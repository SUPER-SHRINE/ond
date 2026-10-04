use super::super::tests::{Fixture, machine};
use kagura::{Bus, FaultCode};
use ram::Ram;
pub const INPUT: u32 = 0x20000000;
pub const OUTPUT: u32 = INPUT + 256;
pub const SENTINEL: u32 = 0xdeadbeef;

#[derive(Clone, Debug)]
pub enum End {
    Return,
    Fault(FaultCode),
}
pub struct Case<'a> {
    pub id: &'a str,
    pub specs: &'a [&'a str],
    pub inputs: &'a [u32],
    /// Output words: traces and final values, or sentinels expected after fault.
    pub output: &'a [u32],
    pub end: End,
    pub steps: usize,
}
pub fn config() -> crate::ProjectConfig {
    crate::ProjectConfig {
        ram_size: 8 * 1024 * 1024,
        stack_size: 1024 * 1024,
    }
}
pub fn project(files: &[(&str, &str)]) -> ond_compiler_core::mir::Project {
    let main = files
        .iter()
        .find(|(p, _)| *p == "main.ond")
        .expect("main source");
    let fixture = Fixture::new(main.1);
    let mut paths = std::collections::BTreeSet::new();
    for (path, source) in files {
        assert!(
            path.ends_with(".ond")
                && path
                    .split('/')
                    .all(|s| !s.is_empty() && s != "." && s != "..")
        );
        assert!(!path.contains(['\\', ':']) && paths.insert(*path));
        let target = fixture.0.join(path);
        std::fs::create_dir_all(target.parent().unwrap()).unwrap();
        std::fs::write(target, source).unwrap();
    }
    fixture.core()
}
pub fn compile(files: &[(&str, &str)]) -> crate::linker::LinkedImage {
    crate::test_support::build_image(&project(files), config()).unwrap()
}
pub fn check(bytes: &crate::linker::LinkedImage, case: &Case<'_>) -> Result<(), String> {
    if case.id.is_empty()
        || case.specs.is_empty()
        || case.steps == 0
        || case.inputs.len() > 64
        || case.output.len() > 64
    {
        return Err("invalid case metadata".into());
    }
    let (mut cpu, mut bus, top) = machine(bytes);
    bus.map_device(INPUT, 512, Ram::new(512)).unwrap();
    for (i, value) in case.inputs.iter().enumerate() {
        bus.write32(INPUT + i as u32 * 4, *value).unwrap();
    }
    for i in 0..64 {
        bus.write32(OUTPUT + i * 4, SENTINEL).unwrap();
    }
    let mut ended = None;
    for _ in 0..case.steps {
        if let Err(e) = cpu.step(&mut bus) {
            ended = Some(End::Fault(e.code()));
            break;
        }
        if bus.read32(0xffff8040).unwrap() != SENTINEL {
            if bus.read32(0xffff8040).unwrap() != 0 {
                return Err(format!("{}: nonzero exit", case.id));
            }
            if cpu.reg(14) != top {
                return Err(format!("{}: SP not restored", case.id));
            }
            for r in 7..=11 {
                if cpu.reg(r) != 0xabcd0000 + r as u32 {
                    return Err(format!("{}: callee-save r{r}", case.id));
                }
            }
            ended = Some(End::Return);
            break;
        }
    }
    let matches = match (&ended, &case.end) {
        (Some(End::Return), End::Return) => true,
        (Some(End::Fault(a)), End::Fault(b)) => {
            a == b && bus.read32(0xffff8040).unwrap() == SENTINEL
        }
        _ => false,
    };
    if !matches {
        return Err(format!(
            "{} {:?}: expected {:?}, observed {:?} (step limit {})",
            case.id, case.specs, case.end, ended, case.steps
        ));
    }
    let actual = (0..case.output.len())
        .map(|i| bus.read32(OUTPUT + i as u32 * 4).unwrap())
        .collect::<Vec<_>>();
    if actual != case.output {
        return Err(format!(
            "{} {:?}: expected {:?}, observed {:?}",
            case.id, case.specs, case.output, actual
        ));
    }
    Ok(())
}
pub fn run(bytes: &crate::linker::LinkedImage, case: Case<'_>) {
    check(bytes, &case).unwrap();
}

#[test]
fn runner_rejects_wrong_values_faults_and_step_limits() {
    let bytes = compile(&[(
        "main.ond",
        &format!("package main\nfunc main(){{store32({OUTPUT},7)}}"),
    )]);
    let mut case = Case {
        id: "runner.self-check",
        specs: &["HARNESS"],
        inputs: &[],
        output: &[7],
        end: End::Return,
        steps: 1000,
    };
    assert!(check(&bytes, &case).is_ok());
    case.output = &[8];
    assert!(check(&bytes, &case).unwrap_err().contains("observed"));
    case.output = &[7];
    case.end = End::Fault(FaultCode::InvalidInstruction);
    assert!(check(&bytes, &case).is_err());
    case.end = End::Return;
    case.steps = 1;
    assert!(check(&bytes, &case).unwrap_err().contains("step limit"));
}
