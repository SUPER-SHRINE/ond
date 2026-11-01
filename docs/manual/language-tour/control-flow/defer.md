# defer

`defer`は現在の関数が正常に戻るときに実行する文を登録します。\
\
複数登録した文は後入れ先出しで実行されます。

```ond title="main.ond"
package main

func main() {
    var first: u32 = 1
    defer {
        use(first)
    }
    defer {
        use(first + 1)
    }
}

func use(value: u32) {
    _ = value
}
```

## 実行順

正常にreturnすると、後から登録した`use(first + 1)`が先に実行されます。

## 登録できる文

defer文はloop内に置けず、defer本体にもdefer文を置けません。\
\
fault時に後続deferが実行される保証はありません。

共通manifestとcompile/linkは[最初のプログラム](../../getting-started/first-program.md)、deferの動作は[文仕様](../../../../specs/ond/language/v1/statements.md)を参照してください。
