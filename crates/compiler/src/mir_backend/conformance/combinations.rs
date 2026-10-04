// Cross-feature cases are kept separate from the shared scalar/evaluation data.
use super::super::{frame::Frame, layout, *};
use super::runner::*;
use kagura::{Bus, FaultCode};

fn enlarge(project: &mut mir::Project) {
    for f in project.packages.iter_mut().flat_map(|p| &mut p.functions) {
        let Some(template) = f.locals.first().cloned() else {
            continue;
        };
        while f.locals.len() < 10000 {
            let mut l = template.clone();
            l.id = mir::LocalId(f.locals.len() as u32);
            l.ty = mir::TypeId::U32;
            l.kind = mir::LocalKind::Temporary;
            l.name = None;
            f.locals.push(l);
        }
    }
    mir::validate_project(project).unwrap();
}

#[test]
fn far_calls_large_frames_hidden_returns_stack_arguments_and_helpers_coexist() {
    let source=format!("package main\n
        type S struct{{a:u32;b:u32;}}
        var G:*S
        func initialize()->*S{{return new(S)}}
        func leaf(a:u32,b:u32,c:u32,d:u32,e:u32,f:u32,g:S)->(S,u32){{
            var sum=a+b+c+d+e+f+g.a;return S{{a:sum,b:g.b}},sum/3
        }}
        func recur(s:S,n:u32)->S{{if n==0{{return s}};s.a=s.a+1;var q=(s.a as f32)/2.0; s.b=q as u32;return recur(s,n-1)}}
        func padding(){{var buf:[2000]u8;buf[1999]=1}}
        func main(){{G=initialize();var s=S{{a:7,b:8}};var t,q=leaf(1,2,3,4,5,6,s);var r=recur(t,3);
            G.a=r.a;store32({OUTPUT},t.a);store32({},t.b);store32({},q);store32({},r.a);store32({},r.b);store32({},s.a);free(G)}}",
        OUTPUT+4,OUTPUT+8,OUTPUT+12,OUTPUT+16,OUTPUT+20);
    let mut p = project(&[("main.ond", &source)]);
    enlarge(&mut p);
    let objects = codegen_objects(&p).unwrap();
    let mut addresses = std::collections::BTreeMap::new();
    let mut base = 0;
    for o in &objects {
        for s in &o.symbols {
            if s.section == Some(SectionKind::Text) {
                addresses.insert(s.name.clone(), base + s.offset);
            }
        }
        base += o
            .sections
            .iter()
            .find(|s| s.kind == SectionKind::Text)
            .unwrap()
            .memory_size;
    }
    assert!(addresses["main.main"] - addresses["main.leaf"] > 131072);
    // Leaf's div helper is also far: the inserted text lies between the call and runtime object.
    for name in ["main.leaf", "main.initialize"] {
        let leaf = &objects[0].symbols.iter().find(|s| s.name == name).unwrap();
        assert!(objects[0].relocations.iter().any(|r| {
            r.kind == RelocationKind::CallSlot
                && r.offset >= leaf.offset
                && r.offset < leaf.offset + leaf.size
                && addresses
                    .get(&r.target)
                    .is_some_and(|a| i64::from(*a) - i64::from(r.offset) > 131072)
        }));
    }
    let bytes = crate::test_support::build_image(&p, config()).unwrap();
    run(
        &bytes,
        Case {
            id: "ABI.far-large-aggregate-helper",
            specs: &["CALL", "MEM-02", "OUT"],
            inputs: &[],
            output: &[28, 8, 9, 31, 15, 7],
            end: End::Return,
            steps: 2_000_000,
        },
    );
}

fn size(p: &mir::Project, name: &str) -> u32 {
    let f = p.packages[0]
        .functions
        .iter()
        .find(|f| f.name == name)
        .unwrap();
    let signature = mir::FunctionType {
        parameters: f
            .parameters
            .iter()
            .map(|id| f.locals[id.0 as usize].ty)
            .collect(),
        returns: f.returns.clone(),
    };
    let abi = layout::abi(&p.types, &signature, f.span).unwrap();
    Frame::new(f, &p.types, &abi).unwrap().size
}

#[test]
fn large_recursive_frames_fit_exact_stack_then_fault_one_level_later() {
    let source = format!(
        "package main\nfunc walk(n:u32)->u32{{if n==0{{return 7}};return walk(n-1)+1}}\nfunc main(){{var n=walk(load32({INPUT}));store32({OUTPUT},n)}}"
    );
    let mut p = project(&[("main.ond", &source)]);
    enlarge(&mut p);
    let exact = size(&p, "main.main") + 3 * size(&p, "main.walk");
    assert!(exact > 131072);
    let mut cfg = config();
    cfg.stack_size = exact;
    let bytes = crate::test_support::build_image(&p, cfg).unwrap();
    run(
        &bytes,
        Case {
            id: "FRAME.exact-fit",
            specs: &["MEM-02"],
            inputs: &[2],
            output: &[9],
            end: End::Return,
            steps: 200_000,
        },
    );
    run(
        &bytes,
        Case {
            id: "FRAME.next-call-fault",
            specs: &["MEM-03"],
            inputs: &[3],
            output: &[SENTINEL],
            end: End::Fault(FaultCode::InvalidInstruction),
            steps: 200_000,
        },
    );
    // Check the physical guard word below stack, not just the absent final output.
    let (mut cpu, mut bus, top) = super::super::tests::machine(&bytes);
    bus.map_device(INPUT, 512, ram::Ram::new(512)).unwrap();
    bus.write32(INPUT, 3).unwrap();
    let bottom = top - exact;
    bus.write32(bottom - 4, 0x12345678).unwrap();
    let fault = (0..200_000).find_map(|_| cpu.step(&mut bus).err()).unwrap();
    assert_eq!(fault.code(), FaultCode::InvalidInstruction);
    assert_eq!(cpu.reg(14), bottom);
    assert_eq!(bus.read32(bottom - 4).unwrap(), 0x12345678);
    // Artificially small SP selects the subtraction-underflow guard.
    let (mut cpu, mut bus, _) = super::super::tests::machine(&bytes);
    cpu.set_reg(14, 0x2000);
    bus.write32(0x1ffc, 0xabcdef01).unwrap();
    let fault = (0..1000).find_map(|_| cpu.step(&mut bus).err()).unwrap();
    assert_eq!(fault.code(), FaultCode::InvalidInstruction);
    assert_eq!(cpu.reg(14), 0x2000);
    assert_eq!(bus.read32(0x1ffc).unwrap(), 0xabcdef01);
}
