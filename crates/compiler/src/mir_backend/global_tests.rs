//! Real CPU tests for zero storage, static relocations and MIR-ordered startup.
use super::{
    tests::{Fixture, execute, run},
    *,
};
use std::fs;

#[test]
fn global_objects_use_checked_natural_bss_layout_and_abs32_relocations() {
    let f = Fixture::new(
        "package main\nvar a:u8=1\nvar b:u16=2\nvar c:u8=3\nvar z:struct{}\nfunc main(){b=9}",
    );
    let p = f.core();
    let objects = codegen_objects(&p).unwrap();
    let o = &objects[0];
    let bss = o
        .sections
        .iter()
        .find(|s| s.kind == SectionKind::Bss)
        .unwrap();
    assert!(bss.data.is_empty());
    assert_eq!(bss.memory_size, 4);
    for (name, offset, size, section) in [
        ("main.a", 0, 1, SectionKind::Data),
        ("main.b", 2, 2, SectionKind::Data),
        ("main.c", 4, 1, SectionKind::Data),
        ("main.z", 0, 0, SectionKind::Bss),
    ] {
        let s = o.symbols.iter().find(|s| s.name == name).unwrap();
        assert_eq!((s.offset, s.size), (offset, size));
        assert_eq!(s.section, Some(section),);
    }
    assert!(p.packages[0].initializer.is_none());
    assert!(
        o.relocations
            .iter()
            .any(|r| r.kind == RelocationKind::Abs32 && r.target == "main.b")
    );
}

#[test]
fn string_literals_and_aggregate_constants_have_stable_rodata_addresses() {
    run(
        "func text()->*u8{return &\"abc\"[0]}\nfunc calc()->u32{var p=text();return p[0] as u32*10000+p[1] as u32*100+p[2] as u32}",
        "calc()",
        979899,
    );
    run(
        "const Data=[3]u16{10,20,30}\nfunc data()->*u16{return &Data[0]}\nfunc calc()->u32{var p=data();return Data[1] as u32+p[2] as u32}",
        "calc()",
        50,
    );

    let fixture = Fixture::new(
        "package main\nconst Message=\"abc\"\nfunc pointer()->*u8{return &Message[0]}\nfunc main(){}",
    );
    let project = fixture.core();
    let objects = codegen_objects(&project).unwrap();
    let object = objects.iter().find(|object| object.name == ".").unwrap();
    let symbol = object
        .symbols
        .iter()
        .find(|symbol| symbol.name == "main.Message")
        .unwrap();
    assert_eq!(symbol.section, Some(SectionKind::Rodata));
    assert_eq!(
        symbol.size, 3,
        "the logical string size excludes its sentinel"
    );
    let rodata = object
        .sections
        .iter()
        .find(|section| section.kind == SectionKind::Rodata)
        .unwrap();
    assert_eq!(
        &rodata.data[symbol.offset as usize..symbol.offset as usize + 3],
        b"abc"
    );
    assert_eq!(rodata.data[symbol.offset as usize + 3], 0);
    assert!(object.relocations.iter().any(|relocation| {
        relocation.kind == RelocationKind::Abs32 && relocation.target == "main.Message"
    }));
}

#[test]
fn source_location_builtins_use_shared_logical_paths_and_one_based_lines() {
    run("func line()->u32{\nreturn thisLine()\n}", "line()", 3);
    run(
        "func file()->*u8{return thisFile()}\nfunc calc()->u32{var p=file();return p[0] as u32*10000+p[4] as u32*100+p[8] as u32}",
        "calc()",
        1_094_600,
    );

    let fixture = Fixture::new(
        "package main\nfunc first()->*u8{return thisFile()}\nfunc second()->*u8{return thisFile()}\nfunc main(){}",
    );
    let objects = codegen_objects(&fixture.core()).unwrap();
    let object = objects.iter().find(|object| object.name == ".").unwrap();
    let paths = object
        .symbols
        .iter()
        .filter(|symbol| symbol.name.contains(".__ond_ro_") && symbol.size == 8)
        .collect::<Vec<_>>();
    assert_eq!(paths.len(), 2);
    assert_eq!(paths[0].offset, paths[1].offset);

    let fixture = Fixture::new("package main\nimport \"shared\"\nfunc main(){shared.File()}");
    fs::create_dir(fixture.0.join("shared")).unwrap();
    fs::write(
        fixture.0.join("shared/shared.ond"),
        "package shared\nfunc File()->*u8{return thisFile()}",
    )
    .unwrap();
    let objects = codegen_objects(&fixture.core()).unwrap();
    let object = objects
        .iter()
        .find(|object| object.name == "shared")
        .unwrap();
    let rodata = object
        .sections
        .iter()
        .find(|section| section.kind == SectionKind::Rodata)
        .unwrap();
    assert!(
        rodata
            .data
            .windows(b"shared/shared.ond\0".len())
            .any(|bytes| bytes == b"shared/shared.ond\0")
    );
}

#[test]
fn null_terminated_literal_storage_shares_exact_and_suffix_values() {
    let fixture = Fixture::new(
        "package main\nfunc long()->*u8{return &\"foobar\"[0]}\nfunc short()->*u8{return &\"bar\"[0]}\nfunc same()->*u8{return &\"foobar\"[0]}\nfunc main(){}",
    );
    let objects = codegen_objects(&fixture.core()).unwrap();
    let object = objects.iter().find(|object| object.name == ".").unwrap();
    let long = object
        .symbols
        .iter()
        .filter(|symbol| symbol.name.contains(".__ond_ro_") && symbol.size == 6)
        .collect::<Vec<_>>();
    let short = object
        .symbols
        .iter()
        .find(|symbol| symbol.name.contains(".__ond_ro_") && symbol.size == 3)
        .unwrap();
    assert_eq!(long.len(), 2);
    assert_eq!(long[0].offset, long[1].offset);
    assert_eq!(short.offset, long[0].offset + 3);
    let rodata = object
        .sections
        .iter()
        .find(|section| section.kind == SectionKind::Rodata)
        .unwrap();
    assert_eq!(
        &rodata.data[long[0].offset as usize..long[0].offset as usize + 7],
        b"foobar\0"
    );
}

#[test]
fn exported_aggregate_constant_address_links_across_packages() {
    let fixture = Fixture::new(
        "package main\nimport \"shared\"\nfunc main(){var p=&shared.Data[0];store32(4294934608,p[0] as u32+p[1] as u32)}",
    );
    fs::create_dir(fixture.0.join("shared")).unwrap();
    fs::write(
        fixture.0.join("shared/shared.ond"),
        "package shared\nconst Data=[2]u8{20,22}",
    )
    .unwrap();
    assert_eq!(
        execute(&crate::test_support::compile_image(&fixture.0).unwrap()),
        42
    );
}

#[test]
fn direct_writes_to_static_constants_and_string_literals_are_rejected() {
    for statement in ["Data[0]=9", "\"abc\"[0]=9"] {
        let fixture = Fixture::new(&format!(
            "package main\nconst Data=[1]u8{{1}}\nfunc main(){{{statement}}}"
        ));
        let error = crate::pipeline::compile_project(&fixture.0).unwrap_err();
        assert!(error.to_string().contains("cannot assign to a constant"));
    }
}

#[test]
fn global_capacity_overflow_is_an_error_not_a_large_host_allocation() {
    for (declaration, expected) in [
        ("var a:[65536]u8", "does not fit"),
        ("var a:[4294967295]u8", "overflow"),
        ("var a:[2147483648]u8\nvar b:[2147483648]u8", "overflow"),
    ] {
        let f = Fixture::new(&format!("package main\n{declaration}\nfunc main(){{}}"));
        let err = crate::test_support::compile_image(&f.0).unwrap_err();
        assert!(err.to_string().contains(expected), "{err}");
    }
}

#[test]
fn combined_package_bss_overflow_is_diagnosed_by_linker() {
    let f = Fixture::new("package main\nimport \"other\"\nvar A:[2147483648]u8\nfunc main(){}");
    fs::create_dir(f.0.join("other")).unwrap();
    fs::write(
        f.0.join("other/other.ond"),
        "package other\nvar B:[2147483648]u8",
    )
    .unwrap();
    let err = crate::test_support::compile_image(&f.0).unwrap_err();
    assert!(
        err.to_string().contains("linked section size overflow"),
        "{err}"
    );
}
