# 公開範囲

package外から直接参照するtop-level宣言名とstruct field名はASCII大文字で始めます。

```text
app/
├─ ond.toml
├─ main.ond
└─ profile/
   └─ profile.ond
```

```ond title="profile/profile.ond"
package profile

type Profile struct {
    Name: [3]u8
    age: u32
}

func NewProfile() -> Profile {
    return Profile{Name: "Aki"}
}
```

```ond title="main.ond"
package main

import "app/profile"

func main() {
    var record: profile.Profile = profile.NewProfile()
    _ = record.Name[0]
}
```

## package-level名

`Profile`と`NewProfile`は先頭がASCII大文字なので、importした別packageから直接参照できます。\
実行projectのroot packageはimportできないため、exported APIはsubpackageに置いています。

## struct field

`Name`は公開fieldで、`main`から参照できます。\
`age`は非公開fieldなので`profile` package内だけで参照できます。\
関数内のlocal名はこの公開規則の対象ではありません。

共通manifestとcompile/linkは[最初のプログラム](../../getting-started/first-program.md)、可視性規則は[名前とscope仕様](../../../../specs/ond/language/v1/names-and-scopes.md#packageとimport)と[型仕様](../../../../specs/ond/language/v1/types.md#型同一性)を参照してください。
