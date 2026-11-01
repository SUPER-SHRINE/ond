# scope

scopeは宣言名を参照できる範囲です。\
関数内のblockにlocal変数を宣言できます。

```ond title="main.ond"
package main

var value: u32 = 1

func main() {
    var value: u32 = 2
    {
        var value: u32 = 3
        _ = value
    }
    _ = value
}
```

## package scope

package-levelの`value`は同じpackageのfileから参照できます。\
関数内の宣言は外側のpackage宣言をshadowします。

## block scope

内側blockの`value`はそのblock内で外側の名前をshadowします。\
blockを出ると外側の`value`が再び参照されます。

共通manifestとcompile/linkは[最初のプログラム](../../getting-started/first-program.md)、scope規則は[名前とscope仕様](../../../../specs/ond/language/v1/names-and-scopes.md#scopeとshadowing)を参照してください。
