# Built-ins v1
<!-- ond-spec-id: builtins/v1 -->

本章は組み込み関数と専用構文を定義する。

## 1. 一覧
<!-- ond-spec-id: builtins/index -->

### 組み込み関数
<!-- ond-spec-id: builtins/functions -->

| Name | Signature                                                         | Summary |
| --- |-------------------------------------------------------------------| --- |
| `alloc` | `alloc[T](count: u32) -> *T` | 型付きの未初期化 heap 領域を確保する |
| `free` | `free(pointer: *T)` 相当                                            | `alloc` / `new` が返した領域を解放する |
| `thisFile` | `thisFile() -> *u8` | 現在の論理source pathを返す |
| `thisLine` | `thisLine() -> u32` | 現在のsource行番号を返す |

### 組み込み構文
<!-- ond-spec-id: builtins/syntax -->

| Name | Form | Summary |
| --- | --- | --- |
| `new` | `new(T)` | 型 `T` 1 個分の zero-initialized heap 領域を確保する |
| 配列長 `len` | `len(value)`（`value: [N]T`） | 配列長を compile-time constant `u32` として返す |
| layout | `sizeof(T)`, `alignof(T)` | 型の size / alignment を `u32` で返す |
| `trap` | `trap "reason"` | 理由をdebug metadataへ記録し、回復不能なCPU trapで停止する |

配列型の byte size が [Data Layout](../../targets/kagura-v1/machine.md#5-data-layout) の上限を超える場合は、`len`、`sizeof`、`alignof` だけで参照しても compile error とする。\
`new` の型 operand や `alloc[T]` の型 argument にも同じ型の制約を適用する。

## 2. `alloc`
<!-- ond-spec-id: builtins/alloc -->

```ond
var ptr: *Actor = alloc[Actor](4 as u32)
```

- 引数は 1 個だけ必要
- 返り値は `*T`
- `count` は `u32`
- 確保領域、alignment、初期状態、失敗とzero-sizeの結果は[メモリ管理](./execution.md#メモリ管理)に従う

## 3. `free`
<!-- ond-spec-id: builtins/free -->

```ond
free(ptr)
free(nil)
```

- 引数は 1 個だけ必要
- 返り値はない
- 有効な引数と解放結果は[メモリ管理](./execution.md#メモリ管理)に従う

## 4. `len`
<!-- ond-spec-id: builtins/len -->

### 4.1 配列に対する `len`
<!-- ond-spec-id: builtins/len-arrays -->

```ond
var size: u32 = len(buffer)
```

`len(value)`は[operator解決](./expressions.md#user-defined-operator)によりcustom宣言を先に選び、builtin配列lenを選んだ場合に以下を適用する。

- `buffer: [N]T` に対して使える
- 結果は配列長 `N` を表す compile-time constant `u32`
- 文字列literalも`[N]u8`なので同じ規則を使う
  - 例えば`len("日本語")`は`9`である
- operand は通常どおり一度評価する

配列lenはoperandの型情報から解決するbuiltin構文である。\
operandに通常callを含む`len(f())`は、const初期値など定数値を要求する位置ではcompile errorとなる。

operand 内で評価される定数変換は範囲検査の対象となり、表現範囲外なら compile error とする。\
`&&` / `||` の短絡評価で未評価となる部分には、通常の短絡規則を適用する。

custom宣言のsignatureと結果型は[operator宣言](./expressions.md#user-defined-operator)を定義元とする。\
custom宣言もbuiltin配列lenも成立しない入力はcompile errorとなる。

## 5. `new`
<!-- ond-spec-id: builtins/new -->

```ond
var ptr: *[4]u8 = new([4]u8)
```

- `new(T)` は関数ではなく専用構文
- 返り値は `*T`
- `new([4]u8)`の返り値は`*[4]u8`であり、assignment等では通常の型同一性を検査する
- 確保領域、初期化、失敗とzero-sizeの結果は[メモリ管理](./execution.md#メモリ管理)に従う

## 6. `thisFile`
<!-- ond-spec-id: builtins/this-file -->

```ond
report(thisFile(), thisLine())
```

- 引数は0個
- `thisFile()` 自身を含むsource fileの論理pathを、null終端された読み取り専用byte列への`*u8`として返す
- project所有のfileはproject rootからの相対pathを使う
root直下のfileはfile名だけとなり、subpackageでもproject名を含めない
- 追加library rootまたは依存projectから渡されるfileは、そのpackageのlogical import pathとfile名を`/`で連結する
- path separatorはhost OSによらず`/`とする
- 指し示すstorageの寿命はプログラム全体とし、書き込んではならない
- 同じ論理pathのstorageは同一Object内で共有してよい

例えばproject rootの`main.ond`では`"main.ond\000"`相当、project `game`内の`draw/render.ond`では`"draw/render.ond\000"`相当のstorageを指す。\
追加library package `graphics/draw`の`render.ond`では`"graphics/draw/render.ond\000"`相当となる。

## 7. `thisLine`
<!-- ond-spec-id: builtins/this-line -->

- 引数は0個
- `thisLine()` 自身の先頭tokenが置かれた、1始まりのsource行番号を`u32`で返す
- 行番号はこのsource tokenの位置で決まる

## 8. `trap`
<!-- ond-spec-id: builtins/trap -->

```ond
trap "player index is out of range"
trap `raw reason`
```

- `trap`はCPU trapで実行を終了するstatementである
- operandはquoted/raw string literalを1個取り、それ以外の入力はcompile errorとなる
- literalはUTF-8として有効でなければならない
  - escapeは通常の文字列literalと同じ規則でdecodeする
- 理由はObject内のdebug metadataへ格納する
- linkerは理由を`.onddebug`へ移し、実行命令は引数なしのCPU trapとする

## 9. 名前解決上の注意
<!-- ond-spec-id: builtins/name-resolution -->

binding名は[scopeの組み込み名規則](./names-and-scopes.md#scopeとshadowing)に従い検査する。\
field名等のmember名には組み込み名の綴りを使える。

## 10. 入力の成立条件
<!-- ond-spec-id: builtins/compile-errors -->

引数の個数・型・結果は各組み込みの[alloc](#2-alloc)、[free](#3-free)、[len](#4-len)、[new](#5-new)、[thisFile](#6-thisfile)、[thisLine](#7-thisline)の定義を適用する。\
custom lenのsignatureは[operator宣言](./expressions.md#user-defined-operator)、trapのoperandは[trap](#8-trap)を定義元とし、不成立の入力はcompile errorとなる。

## 11. 関連仕様
<!-- ond-spec-id: builtins/related-specifications -->

- [Ond Language v1](./README.md)
- [Ond Debug Metadata v1](../../debug-metadata/v1.md)
- [適合と実装追跡](../../conformance/implementation-index.md)
- [Ond for Kagura Machine Profile v1](../../targets/kagura-v1/machine.md)
