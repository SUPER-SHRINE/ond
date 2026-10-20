use crate::test_support;

#[test]
fn qualified_fields_preserve_constant_roots_and_local_shadowing() {
    for declaration in [
        "const C=S{Value:1}\nfunc main(){C.Value=2}",
        "func main(){const C=S{Value:1};C.Value=2}",
    ] {
        assert!(
            test_support::compile(
                &format!("package main\ntype S struct{{Value:u32}}\n{declaration}\n"),
                None
            )
            .is_err()
        );
    }
    assert!(
        test_support::compile(
            "package main\nimport \"test/shared\"\nfunc main(){shared.C.Value=2}\n",
            Some("package shared\ntype S struct{Value:u32}\nconst C=S{Value:1}\n")
        )
        .is_err()
    );
    test_support::compile("package main\ntype S struct{Value:u32}\nconst C=S{Value:1}\nfunc main(){var C:S;C.Value=2}\n",None).expect("a mutable local shadows the package constant");
}

#[test]
fn simple_setter_resolves_typed_operands_before_literal_contexts() {
    for body in [
        "var i:u8;b[i]=1",
        "b[0 as u8]=1",
        "var x:u32;b[0 as u16]=x",
        "b[0]=1",
    ] {
        test_support::compile(&format!("package main\ntype B [1]u32\noperator(b:*B) []=(i:u16,x:u16){{}}\nfunc main(){{var b:B;{body}}}\n"),None).expect("typed operands select a matching custom or builtin setter");
    }
}

#[test]
fn scalar_compound_assignment_applies_custom_priority_and_result_type() {
    for target in ["n", "record.Value", "*pointer"] {
        for operation in [format!("{target}+=1"), format!("{target}++")] {
            let source = format!(
                "package main\ntype N i32\ntype Record struct{{Value:N}}\noperator(n:N)+(x:N)->bool{{return true}}\nfunc main(){{var n:N;var record:Record;pointer:=&n;{operation}}}\n"
            );
            let error = test_support::compile(&source, None)
                .expect_err("selected custom result must match storage");
            assert!(!format!("{error:?}").contains("internal MIR"));
        }
    }
}

#[test]
fn custom_setter_cannot_assign_through_a_constant_root() {
    let prefix =
        "package main\ntype B [1]u32\noperator(b:*B) []=(i:u32,x:u32){(b as u32 as *u32)[i]=x}\n";
    assert!(
        test_support::compile(
            &format!("{prefix}const X=B{{1}}\nfunc main(){{X[0]=9}}\n"),
            None
        )
        .is_err()
    );
    test_support::compile(
        &format!("{prefix}func main(){{var x=B{{1}};x[0]=9}}\n"),
        None,
    )
    .expect("mutable custom setter remains valid");
}

#[test]
fn unused_generic_bodies_obey_declaration_scope_rules() {
    let declarations = [
        "func unused[T](x:T){var x:T}",
        "func unused[T](x:T){const y=1;var y:T}",
        "func unused[T](x:T){var y:T;const y=1}",
        "func unused[T](x:T){x:=x}",
        "func unused[T](x:T){_:=x}",
        "func unused[T](x:T){y,y:=x,x}",
        "func unused[T](x:T){const y=1;y,z:=1,x;_=z}",
        "func unused[T](x:T,x:T){}",
        "type Box[T] struct{Value:T}\nfunc(b:Box[T]) unused(){var b:Box[T]}",
    ];
    for declaration in declarations {
        assert!(
            test_support::compile(
                &format!("package main\n{declaration}\nfunc main(){{}}\n"),
                None
            )
            .is_err(),
            "{declaration}"
        );
    }
    test_support::compile("package main\nfunc unused[T](x:T){x,y:=x,x;_=y;{var x:T;_=x};{x:=x;_=x};const z=1;{z,w:=x,x;_=z;_=w}}\nfunc main(){}\n",None).expect("nested shadowing and same-block variable reuse remain valid");
}

#[test]
fn nested_index_assignment_preserves_readonly_and_setter_rules() {
    for target in ["A[idx[i]]", "A[(idx[i])]"] {
        let source = format!(
            "package main\nconst A=[1]u32{{1}}\nfunc main(){{idx:=[1]u32{{0}};var i:u32;{target}=5}}\n"
        );
        let error = test_support::compile(&source, None).expect_err("constant array is readonly");
        assert!(!format!("{error:?}").contains("internal MIR"));
    }
    let source = "package main\ntype T struct{}\noperator(t:T) []=(index:u32,value:u32){}\nfunc main(){var t:T;idx:=[1]u32{0};var i:u32;t[idx[i]]=5}\n";
    test_support::compile(source, None).expect("setter-only nested index");
}

#[test]
fn unused_generic_nested_index_validates_value_names() {
    for usage in ["_=a[missing[i]]", "a[missing[i]]()"] {
        let source =
            format!("package main\nfunc unused[T](a:*T){{var i:u32;{usage}}}\nfunc main(){{}}\n");
        let error = test_support::compile(&source, None).expect_err("undefined nested index base");
        assert!(format!("{error:?}").contains("missing"));
    }
    test_support::compile("package main\ntype Box[T] struct{Value:T}\nfunc Id[T](x:T)->T{return x}\nfunc unused[T](x:Box[T]){var f:func(Box[T])->Box[T]=Id[Box[T]];_=f}\nfunc main(){var b:Box[u8];var f:func(Box[u8])->Box[u8]=Id[Box[u8]];_=f(b)}\n",None).expect("nested type argument remains a generic function value");
}

#[test]
fn builtin_unary_results_match_requested_type() {
    for op in ["-", "+", "^"] {
        for body in [
            format!("func main(){{var x:u8;var y:i32={op}x;_=y}}"),
            format!("func f(x:u8)->i32{{return {op}x}}\nfunc main(){{}}"),
            format!("func f(x:i32){{}}\nfunc main(){{var x:u8;f({op}x)}}"),
        ] {
            let error = test_support::compile(&format!("package main\n{body}\n"), None)
                .expect_err("unary result retains operand type");
            assert!(!format!("{error:?}").contains("internal MIR"), "{error:?}");
        }
    }
}

#[test]
fn source_information_calls_keep_runtime_value_boundaries() {
    for expression in ["thisLine()", "thisLine()+1", "[1]u32{thisLine()}"] {
        assert!(
            test_support::compile(
                &format!("package main\nconst X={expression}\nfunc main(){{}}\n"),
                None
            )
            .is_err(),
            "{expression}"
        );
    }
    let source = format!(
        "package main\n{}func observe()->(u8,u32){{return thisLine() as u8,thisLine()+4294967295}}\nfunc main(){{}}\n",
        "\n".repeat(260)
    );
    test_support::compile(&source, None).expect("source builtin operations are runtime operations");
}

#[test]
fn compound_index_resolves_write_after_arithmetic_and_updates() {
    for update in ["+=1", "++", "--"] {
        let source = format!(
            "package main\ntype B [1]u32\noperator(b:*B) [](i:u8)->u32{{return 2}}\noperator(b:*B) []=(i:u16,v:bool){{}}\nfunc main(){{var b:B;b[0]{update}}}\n"
        );
        test_support::compile(&source, None)
            .expect("inapplicable setter does not constrain read index");
        let source = format!(
            "package main\ntype B struct{{Value:u32}}\noperator(b:*B) [](i:u8)->u32{{return b.Value}}\noperator(b:*B) []=(i:u8,v:u32){{b.Value=v}}\nfunc main(){{var b:B;b[0]{update}}}\n"
        );
        test_support::compile(&source, None).expect("custom index updates share compound lowering");
    }
}

#[test]
fn compound_index_keeps_literal_conflicts_readonly_and_result_checks() {
    for body in ["var b:B;b[0]+=1", "const b=B{1};b[0]++"] {
        let source = format!(
            "package main\ntype B [1]u32\noperator(b:*B) [](i:u8)->u32{{return 1}}\noperator(b:*B) []=(i:u16,v:u32){{}}\nfunc main(){{{body}}}\n"
        );
        assert!(test_support::compile(&source, None).is_err());
    }
    let source = "package main\ntype B [1]u32\ntype N u32\noperator(b:*B) [](i:u8)->N{return 1 as N}\noperator(n:N)+(rhs:N)->bool{return true}\nfunc main(){var b:B;b[0]+=1}\n";
    assert!(test_support::compile(source, None).is_err());
}

#[test]
fn builtin_index_does_not_take_context_from_inapplicable_setter() {
    for declaration in [
        "operator(b:*B) []=(i:bool,v:bool){}",
        "operator(b:*B) []=(i:u8,v:u32){}",
    ] {
        let source = format!(
            "package main\ntype B [1]u32\n{declaration}\nfunc main(){{var b:B;b[0]+=1;b[0]++;b[0]--}}\n"
        );
        test_support::compile(&source, None).expect("write context follows its value type");
    }
}

#[test]
fn compound_index_literal_is_checked_only_in_final_context() {
    test_support::compile("package main\ntype B *u32\noperator(b:B) []=(i:u32,v:u32){}\nfunc main(){var b:B;b[2147483648]+=1}\n",None).expect("u32 context accepts a literal beyond i32");
    let source = "package main\ntype B [1]u32\noperator(b:*B) []=(i:u32,v:u32){}\nfunc main(){var b:B;b[2147483648]+=1}\n";
    let error = test_support::compile(source, None).unwrap_err();
    assert!(
        format!("{error:?}").contains("constant index out of bounds"),
        "{error:?}"
    );
    let source = "package main\ntype B *u32\noperator(b:B) []=(i:u8,v:u32){}\nfunc main(){var b:B;b[0]+=1}\n";
    assert!(
        test_support::compile(source, None).is_err(),
        "builtin pointer read requires u32 even with custom setter"
    );
}

#[test]
fn indexed_increment_keeps_numeric_operand_requirement() {
    let prefix = "package main\ntype E struct{}\noperator(e:E)+(v:i32)->E{return e}\ntype B struct{}\noperator(b:*B) [](i:u8)->E{return E{}}\noperator(b:*B) []=(i:u8,v:E){}\n";
    test_support::compile(&format!("{prefix}func main(){{var b:B;b[0]+=1}}\n"), None)
        .expect("general compound custom operator remains valid");
    assert!(
        test_support::compile(&format!("{prefix}func main(){{var b:B;b[0]++}}\n"), None).is_err()
    );
    test_support::compile("package main\ntype N u32\noperator(n:N)+(v:N)->N{return n}\ntype B struct{}\noperator(b:*B) [](i:u8)->N{return 0 as N}\noperator(b:*B) []=(i:u8,v:N){}\nfunc main(){var b:B;b[0]++;b[0]--}\n",None).expect("named numeric index remains updatable");
}

#[test]
fn operator_context_respects_receiver_addressability() {
    test_support::compile("package main\ntype N i32\noperator(n:*N)+(rhs:u8)->N{return 9 as N}\nfunc get()->N{return 1 as N}\nfunc main(){var n:N=get()+1;_=n;var m:N;m+=1}\n",None).expect("nonaddressable receiver uses builtin context");
}

#[test]
fn qualified_generic_probe_respects_value_shadowing() {
    test_support::compile("package main\nimport p \"test/shared\"\ntype S struct{Id:[1]func()->u32}\nfunc one()->u32{return 1}\nfunc main(){var g:func(u32)->u32=p.Id[u32];_=g; p:=S{Id:[1]func()->u32{one}};var i:u32;var f:func()->u32=p.Id[i];_=f()}\n",Some("package shared\nfunc Id[T](x:T)->T{return x}\n")).expect("local value precedes imported generic lookup");
}

#[test]
fn fixed_boolean_results_precede_custom_operand_contexts() {
    for value in [
        "1<2",
        "1==2",
        "!(1==2)",
        "true&&false",
        "false||true",
        "true",
    ] {
        let source = format!(
            "package main\ntype B [1]bool\noperator(b:*B) []=(i:u8,v:u8){{}}\nfunc main(){{var b:B;b[0]={value}}}\n"
        );
        test_support::compile(&source, None)
            .expect("fixed bool result makes numeric setter inapplicable");
    }
}

#[test]
fn indexed_float_updates_use_the_selected_numeric_one() {
    for body in [
        "var a:[1]f32;a[0]++;a[0]--",
        "var a:[1]F;a[0]++;a[0]--",
        "var a:[1]f32;var p:*f32=&a[0];p[0]++;p[0]--",
    ] {
        test_support::compile(
            &format!("package main\ntype F f32\nfunc main(){{{body}}}\n"),
            None,
        )
        .expect("builtin float update");
    }
    test_support::compile("package main\ntype F f32\noperator(n:F)+(v:u8)->F{return n}\noperator(n:F)-(v:u8)->F{return n}\ntype B struct{Value:F}\noperator(b:*B) [](i:u8)->F{return b.Value}\noperator(b:*B) []=(i:u8,v:F){b.Value=v}\nfunc main(){var b:B;b[0]++;b[0]--}\n",None).expect("custom numeric RHS determines one type");
}

#[test]
fn indexed_arithmetic_keeps_source_receiver_addressability() {
    let prefix = "package main\ntype N struct{Value:u32}\noperator(n:*N)+(v:u32)->N{return N{Value:n.Value+v}}\ntype B struct{}\noperator(b:*B) [](i:u32)->N{return N{Value:2}}\noperator(b:*B) []=(i:u32,v:N){}\n";
    for operation in ["_=b[0]+1", "b[0]+=1"] {
        assert!(
            test_support::compile(
                &format!("{prefix}func main(){{var b:B;{operation}}}\n"),
                None
            )
            .is_err(),
            "{operation}"
        );
    }
    test_support::compile("package main\ntype N i32\noperator(n:*N)+(v:u8)->N{return 9 as N}\ntype B [1]N\noperator(b:*B) [](i:u32)->N{return 2 as N}\nfunc main(){var b:B;b[0]+=1}\n",None).expect("nonaddressable numeric getter uses builtin RHS context");
}
