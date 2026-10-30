# role

`role`はmanifestで宣言した構成ごとに、選択するsource文を切り替えます。

```ond title="main.ond"
package main

func main() {
    var mode: u32 = 0
    role Debug {
        mode = 1
    }
    role Development {
        mode = 2
    }
    _ = mode
}
```

## 構成を宣言する

manifestには`Debug`と`Development`を宣言します。\
roleを指定しない構成では共通部分だけが選択されます。

## roleを選んでcompileする

`--role Debug`または`--role Development`を指定すると、対応するblockが選択されます。\
各構成は同じsourceから別々にcompileします。

この例では最初のプログラムと同じlink設定を使い、追加role名を宣言します。

```toml title="ond.toml"
[project]
name = "first"

[roles]
names = ["Debug", "Development"]

[link]
startup = "external"
stack-size = 8192
output = "target/first.bin"

[link.ram]
base = 4096
size = 65536

[link.exit]
mode = "loop"
```

roleの選択規則は[言語role仕様](../../../../specs/ond/language/v1/roles.md)、manifestとCLIは[project role仕様](../../../../specs/ond/build/roles.md)を参照してください。
