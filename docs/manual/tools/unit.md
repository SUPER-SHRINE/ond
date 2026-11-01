# Ond unit test runner

`ond-unit` は、登録した関数をKagura環境で実行して結果を確認するコマンドです。\
利用者の通常のOnd projectをcompile、linkし、その実行可能imageを使います。\
設定schemaと実行結果の契約は[ond-unit規範仕様](../../../specs/ond/tools/unit.md)を参照してください。

## projectの例

次の構成では、`cases/check.ond`にある`Check`関数を1件実行します。

```text
sample/
├── ond.toml
├── ond-unit.toml
└── cases/
    └── check.ond
```

`ond.toml`:

```toml
[project]
name = "checks"
```

`cases/check.ond`:

```ond
package cases

func Check() {
    var value = 0x8000 as *u32
    if *value != 42 {
        trap "unexpected value"
    }
}
```

`ond-unit.toml`:

```toml
version = 1

[project]
path = "."

[vm]
ram_size = 65536
stack_size = 4096

[[cases]]
name = "check-initial-value"
package = "checks/cases"
function = "Check"
ram_initial = [{ address = 0x8000, bytes = [42, 0, 0, 0] }]
```

## 実行

project rootで実行します。

```console
ond-unit
```

別の設定fileを使う場合はpathを渡し、ケースを絞る場合は`--case`を繰り返します。

```console
ond-unit ond-unit.toml --case check-initial-value
```

通常のOnd compiler CLIもPATHから利用できる必要があります。\
配布版では、`ond-unit`と同じ0.4.0 releaseの`ond`を使用してください。\
ソースからのbuildではrepository rootで`cargo build -p ond-unit --locked`を実行します。\
MMIOの設定、期待fault、case file分割、実行上限などは[規範仕様](../../../specs/ond/tools/unit.md)を参照してください。
