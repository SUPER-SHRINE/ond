# if

`if`は条件がtrueのときにblockを実行します。\
条件の前にinit文を置き、その結果を判定できます。

```ond title="main.ond"
package main

func TrySomething() -> (u32, bool) {
    return 42, true
}

func main() {
    var count: u32 = 3
    if count > 0 {
        count++
    } else {
        count = 1
    }

    if value, ok := TrySomething(); ok == true {
        _ = value
    } else {
        _ = value
    }
    _ = count
}
```

## 条件とblock

条件式は`bool`を返します。\
trueなら直後のblockを実行します。

## init文で値を受け取る

`if value, ok := TrySomething(); ok == true {}`はinit文でcallの戻り値を受け取り、その後に`ok`を判定します。\
init文で宣言した`value`と`ok`はthenとelseの両blockから使え、`if`の外側には残りません。\
同じif headerでは条件の`ok == true`を`ok`だけにも書けます。\
例えば`if value, ok := TrySomething(); ok {}`と表せます。

## else分岐

条件がfalseなら`else`のblockを実行します。\
`else if`で条件を続けることもできます。

共通manifestとcompile/linkは[最初のプログラム](../../getting-started/first-program.md)、条件式とinit文は[文仕様](../../../../specs/ond/language/v1/statements.md)、変数のscopeは[名前とscope仕様](../../../../specs/ond/language/v1/names-and-scopes.md)を参照してください。
