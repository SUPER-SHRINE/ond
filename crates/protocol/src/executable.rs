//! ONDX version 1 standard executable wire format.
use crate::object::SectionKind;

#[derive(Debug, Clone)]
pub struct Executable {
    pub entry_address: u32,
    pub initial_sp: u32,
    pub sections: Vec<ExecutableSection>,
}

#[derive(Debug, Clone)]
pub struct ExecutableSection {
    pub kind: SectionKind,
    pub file_offset: u32,
    pub memory_address: u32,
    pub memory_size: u32,
    pub alignment: u32,
    pub data: Vec<u8>,
}

fn kind_number(kind: SectionKind) -> u32 {
    match kind {
        SectionKind::Text => 1,
        SectionKind::Rodata => 2,
        SectionKind::Data => 3,
        SectionKind::Bss => 4,
    }
}

impl Executable {
    fn validate(&self) -> Result<u32, String> {
        if !(1..=4).contains(&self.sections.len()) {
            return Err("ONDX section count must be 1..4".into());
        }
        if self.initial_sp == 0 || !self.initial_sp.is_multiple_of(8) {
            return Err("ONDX initial SP must be nonzero and 8-byte aligned".into());
        }
        let table_end = 32 + 24 * self.sections.len() as u32;
        let mut image_size = table_end;
        let mut previous = 0;
        let mut files = Vec::new();
        let mut memories = Vec::new();
        let mut text = None;
        for section in &self.sections {
            let kind = kind_number(section.kind);
            let size =
                u32::try_from(section.data.len()).map_err(|_| "ONDX section size overflow")?;
            let end = section
                .memory_address
                .checked_add(section.memory_size)
                .ok_or("ONDX memory range overflow")?;
            if kind <= previous
                || section.memory_size == 0
                || !section.alignment.is_power_of_two()
                || !section.memory_address.is_multiple_of(section.alignment)
            {
                return Err("invalid ONDX section order, size or alignment".into());
            }
            previous = kind;
            match section.kind {
                SectionKind::Text | SectionKind::Rodata if size != section.memory_size => {
                    return Err("ONDX read-only file and memory sizes differ".into());
                }
                SectionKind::Data if size > section.memory_size => {
                    return Err("ONDX Data file size exceeds memory size".into());
                }
                SectionKind::Bss if size != 0 => return Err("ONDX Bss contains file bytes".into()),
                _ => {}
            }
            if section.kind == SectionKind::Text {
                if section.alignment < 4 || !section.memory_size.is_multiple_of(4) {
                    return Err("invalid ONDX Text alignment or size".into());
                }
                text = Some((section.memory_address, end));
            }
            if size == 0 {
                if section.file_offset != 0 {
                    return Err("empty ONDX section has a file offset".into());
                }
            } else {
                let file_end = section
                    .file_offset
                    .checked_add(size)
                    .ok_or("ONDX file range overflow")?;
                if section.file_offset < table_end {
                    return Err("ONDX section overlaps header/table".into());
                }
                files.push((section.file_offset, file_end));
                image_size = image_size.max(file_end);
            }
            memories.push((section.memory_address, end));
        }
        for ranges in [&mut files, &mut memories] {
            ranges.sort_unstable();
            if ranges.windows(2).any(|pair| pair[0].1 > pair[1].0) {
                return Err("overlapping ONDX section ranges".into());
            }
        }
        if !self.entry_address.is_multiple_of(4)
            || text
                .is_none_or(|(start, end)| self.entry_address < start || self.entry_address >= end)
        {
            return Err("ONDX entry is outside Text or unaligned".into());
        }
        Ok(image_size)
    }

    pub fn read(bytes: &[u8]) -> Result<Self, String> {
        let get = |offset: usize| -> Result<u32, String> {
            bytes
                .get(offset..offset + 4)
                .map(|s| u32::from_le_bytes(s.try_into().unwrap()))
                .ok_or_else(|| "truncated ONDX header/table".into())
        };
        if bytes.get(..4) != Some(b"ONDX") || get(4)? != 1 || get(8)? != 1 || get(12)? != 1 {
            return Err("unsupported ONDX magic/version/target/abi".into());
        }
        let image_size = get(16)?;
        let count = get(28)?;
        if image_size as usize != bytes.len()
            || !(1..=4).contains(&count)
            || 32 + 24 * count > image_size
        {
            return Err("invalid ONDX image/table length".into());
        }
        let mut sections = Vec::new();
        for index in 0..count as usize {
            let offset = 32 + 24 * index;
            let kind = match get(offset)? {
                1 => SectionKind::Text,
                2 => SectionKind::Rodata,
                3 => SectionKind::Data,
                4 => SectionKind::Bss,
                _ => return Err("unknown ONDX section kind".into()),
            };
            let file_offset = get(offset + 4)?;
            let file_size = get(offset + 8)?;
            let file_end = file_offset
                .checked_add(file_size)
                .ok_or("ONDX file range overflow")?;
            let data = bytes
                .get(file_offset as usize..file_end as usize)
                .ok_or("ONDX section exceeds image")?
                .to_vec();
            sections.push(ExecutableSection {
                kind,
                file_offset,
                data,
                memory_address: get(offset + 12)?,
                memory_size: get(offset + 16)?,
                alignment: get(offset + 20)?,
            });
        }
        let image = Self {
            entry_address: get(20)?,
            initial_sp: get(24)?,
            sections,
        };
        if image.validate()? != image_size {
            return Err("ONDX image has trailing bytes".into());
        }
        let mut ranges = image
            .sections
            .iter()
            .filter(|s| !s.data.is_empty())
            .map(|s| {
                (
                    s.file_offset as usize,
                    s.file_offset as usize + s.data.len(),
                )
            })
            .collect::<Vec<_>>();
        ranges.sort_unstable();
        let mut cursor = (32 + 24 * count) as usize;
        for (start, end) in ranges {
            if bytes[cursor..start].iter().any(|b| *b != 0) {
                return Err("nonzero ONDX padding".into());
            }
            cursor = end;
        }
        Ok(image)
    }

    pub fn write(&self) -> Result<Vec<u8>, String> {
        let image_size = self.validate()?;
        let mut bytes = vec![0; image_size as usize];
        bytes[..4].copy_from_slice(b"ONDX");
        for (index, value) in [
            1,
            1,
            1,
            image_size,
            self.entry_address,
            self.initial_sp,
            self.sections.len() as u32,
        ]
        .into_iter()
        .enumerate()
        {
            bytes[4 + index * 4..8 + index * 4].copy_from_slice(&value.to_le_bytes());
        }
        for (index, section) in self.sections.iter().enumerate() {
            let values = [
                kind_number(section.kind),
                section.file_offset,
                section.data.len() as u32,
                section.memory_address,
                section.memory_size,
                section.alignment,
            ];
            for (field, value) in values.into_iter().enumerate() {
                let offset = 32 + index * 24 + field * 4;
                bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
            }
            let offset = section.file_offset as usize;
            bytes[offset..offset + section.data.len()].copy_from_slice(&section.data);
        }
        Ok(bytes)
    }
}
