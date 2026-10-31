# for

`for`には条件付き、3 clause、条件を省略する形があります。

```ond title="main.ond"
package main

func main() {
    var count: u32 = 0
    for count < 3 {
        count++
    }
    _ = count

    for i := 0; i < 10; i++ {
        _ = i
    }

    for i := 0; ; i++ {
        if i == 9 {
            break
        }
    }

    for {
    }
}
```

## 条件で繰り返す

`for count < 3 {}`は各反復の前に条件を評価し、falseになるとloopを終えます。\
body内の`count++`で次の判定に使う値を更新します。

## init、条件、postを使う

`for i := 0; i < 10; i++ {}`では`i := 0`を1回実行し、条件を評価してからbodyを実行します。\
bodyの後に`i++`を実行し、次の条件判定へ進みます。\
`continue`もpostへ進みます。\
initで宣言した`i`は条件、post、bodyで使え、loopの外には残りません。

## 条件を省略する

`for i := 0; ; i++ {}`は条件を省略するため、`break`で抜けるまで繰り返します。\
この例は`i == 9`で`break`します。\
`break`を除くと、カウンターを更新しながら無限に繰り返します。\
`for {}`も無条件loopで、抜ける処理がなければ繰り返し続けます。

共通manifestとcompile/linkは[最初のプログラム](../../getting-started/first-program.md)、loopの規則は[文仕様](../../../../specs/ond/language/v1/statements.md)、initで宣言した変数のscopeは[名前とscope仕様](../../../../specs/ond/language/v1/names-and-scopes.md)を参照してください。
