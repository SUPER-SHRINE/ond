use ond_protocol::executable::Executable;
fn fixture() -> Vec<u8> {
    "4f4e4458 01000000 01000000 01000000 54000000 50000010 00010010 02000000 01000000 50000000 04000000 50000010 04000000 04000000 04000000 00000000 00000000 54000010 04000000 04000000 000000c0".split_whitespace().flat_map(|s| (0..s.len()).step_by(2).map(|i| u8::from_str_radix(&s[i..i+2],16).unwrap()).collect::<Vec<_>>()).collect()
}
fn put(bytes: &mut [u8], offset: usize, value: u32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}
#[test]
fn bin_01_fixed_wire_roundtrip_and_mutations() {
    let w = fixture();
    let image = Executable::read(&w).unwrap();
    assert_eq!(image.entry_address, 0x10000050);
    assert_eq!(image.initial_sp, 0x10000100);
    assert_eq!(image.sections.len(), 2);
    assert_eq!(image.write().unwrap(), w);
    for (offset, value) in [
        (0, 0),
        (4, 2),
        (8, 2),
        (12, 2),
        (28, u32::MAX),
        (56, 5),
        (56, 1),
        (72, 0),
        (32, 2),
        (36, 32),
        (68, 0x10000050),
        (40, 3),
        (52, 3),
        (20, 0x10000051),
        (20, 0x10000054),
        (60, 80),
    ] {
        let mut bad = w.clone();
        put(&mut bad, offset, value);
        assert!(
            Executable::read(&bad).is_err(),
            "offset {offset}, value {value}"
        );
    }
    let mut bad = w.clone();
    bad.pop();
    assert!(Executable::read(&bad).is_err());
    let mut bad = w.clone();
    bad.push(0);
    assert!(Executable::read(&bad).is_err());
    let mut bad = w.clone();
    bad[32..80].rotate_left(24);
    assert!(Executable::read(&bad).is_err());
    let mut bad = w.clone();
    put(&mut bad, 68, 0xfffffffc);
    put(&mut bad, 72, 8);
    assert!(Executable::read(&bad).is_err());
    let mut p = w.clone();
    p.splice(80..80, [0; 4]);
    for (offset, value) in [
        (16, 88),
        (20, 0x10000054),
        (36, 84),
        (44, 0x10000054),
        (68, 0x10000058),
    ] {
        put(&mut p, offset, value);
    }
    assert_eq!(Executable::read(&p).unwrap().write().unwrap(), p);
    p[80] = 1;
    assert!(Executable::read(&p).is_err());
}

#[test]
fn bin_01_reader_rejects_invalid_initial_sp() {
    for initial_sp in [0, 1, 4] {
        let mut bytes = fixture();
        put(&mut bytes, 24, initial_sp);
        assert!(Executable::read(&bytes).is_err(), "initial_sp {initial_sp}");
    }
}

#[test]
fn bin_01_writer_rejects_invalid_initial_sp() {
    for initial_sp in [0, 1, 4] {
        let mut image = Executable::read(&fixture()).unwrap();
        image.initial_sp = initial_sp;
        assert!(image.write().is_err(), "initial_sp {initial_sp}");
    }
}

#[test]
fn bin_02_data_tail_and_empty_data_wire_vectors() {
    let mut d = Vec::new();
    d.extend_from_slice(b"ONDX");
    for value in [
        1u32, 1, 1, 110, 0x10000068, 0x10000100, 3, 1, 104, 4, 0x10000068, 4, 4, 3, 108, 2,
        0x1000006c, 4, 4, 4, 0, 0, 0x10000070, 4, 4,
    ] {
        d.extend(value.to_le_bytes());
    }
    d.extend([0, 0, 0, 0xc0, 0x11, 0x22]);
    let image = Executable::read(&d).unwrap();
    assert_eq!(image.sections[1].data, [0x11, 0x22]);
    assert_eq!(image.sections[1].memory_size, 4);
    assert_eq!(image.write().unwrap(), d);
    let mut empty = fixture();
    put(&mut empty, 56, 3);
    let image = Executable::read(&empty).unwrap();
    assert_eq!(
        image.sections[1].kind,
        ond_protocol::object::SectionKind::Data
    );
    assert!(image.sections[1].data.is_empty());
    assert_eq!(image.write().unwrap(), empty);
    let mut bad = d.clone();
    put(&mut bad, 64, 5);
    assert!(Executable::read(&bad).is_err());
    let mut bad = d.clone();
    put(&mut bad, 60, 104);
    assert!(Executable::read(&bad).is_err());
    let mut bad = d.clone();
    put(&mut bad, 36, u32::MAX);
    assert!(Executable::read(&bad).is_err());
    let mut bad = d.clone();
    put(&mut bad, 28, 0);
    assert!(Executable::read(&bad).is_err());
    let mut bad = d.clone();
    put(&mut bad, 52, 0);
    assert!(Executable::read(&bad).is_err());
    let mut bad = d.clone();
    put(&mut bad, 44, 0x10000069);
    assert!(Executable::read(&bad).is_err());
}
