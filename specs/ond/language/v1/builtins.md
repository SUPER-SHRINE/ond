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
| 配列長 `len` | `len([N]T)` | 配列長を compile-time constant `u32` として返す |
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
- `alignof(T)` で領域を確保する
- 初期値は未規定
- allocation failure 時は `nil`
- `alloc[T](0)` は `nil`

複数の同型要素を持つ連続領域の確保に使う。\
`alloc[u8]`とすればraw byte bufferとして使える。

## 3. `free`
<!-- ond-spec-id: builtins/free -->

```ond
free(ptr)
free(nil)
```

- 引数は 1 個だけ必要
- 返り値はない
- `alloc(...)` または `new(...)` が返した未解放の先頭 pointer を渡す
- `nil`を渡すとallocation状態を保持して正常に完了する

有効な入力と解放結果は[heap契約](../../targets/kagura-v1/machine.md#31-heap-allocator)に従い、利用側が未解放の先頭pointerと生存期間を管理する。

## 4. `len`
<!-- ond-spec-id: builtins/len -->

### 4.1 配列に対する `len`
<!-- ond-spec-id: builtins/len-arrays -->

```ond
var size: u32 = len(buffer)
```

- `buffer: [N]T` に対して使える
- 結果は配列長 `N` を表す compile-time constant `u32`
- 文字列literalも`[N]u8`なので同じ規則を使う
  - 例えば`len("日本語")`は`9`である
- operand は通常どおり一度評価する

配列lenはoperandの型情報から解決するbuiltin構文である。

operand 内で評価される定数変換は範囲検査の対象となり、表現範囲外なら compile error とする。\
`&&` / `||` の短絡評価で未評価となる部分には、通常の短絡規則を適用する。

配列以外ではreceiverに一致する `operator (self: T) len() -> u32` を呼び出す。\
この宣言では戻り値型に `u32` を1個明示し、戻り値の省略、別の型、複数戻り値は未使用でも compile error とする。\
generic operatorにも同じ規則を適用し、`len(value)` の結果は常に単一の `u32` となる。\
一致する宣言がなければ compile error とする。

## 5. `new`
<!-- ond-spec-id: builtins/new -->

```ond
var ptr: *[4]u8 = new([4]u8)
```

- `new(T)` は関数ではなく専用構文
- 返り値は `*T`
- `new([4]u8)`の返り値は`*[4]u8`であり、assignment等では通常の型同一性を検査する
- `T` 1 個分の領域を確保する
- zero value で初期化する
- allocation failure 時は `nil`

`alloc[T]`は未初期化の`T`を複数個格納する領域、`new(T)`はzero初期化した`T`を1個格納する領域の確保に使う。

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

## 10. よくある compile error
<!-- ond-spec-id: builtins/compile-errors -->

- `alloc(...)` は引数 1 個のみ
- `free(...)` は引数 1 個のみ
- `len(...)` は引数 1 個のみ
- `len(...)` の引数は配列、または一致する `operator len` を持つ型
- `operator len` は戻り値型に `u32` を1個明示する必要がある
- `thisFile()` と `thisLine()` は引数を取らない
- `trap` の直後にはUTF-8として有効なstring literalを置く

## 11. 関連仕様
<!-- ond-spec-id: builtins/related-specifications -->

- [Ond Language v1](./README.md)
- [Ond Debug Metadata v1](../../debug-metadata/v1.md)
- [Ond Compiler IR](../../conformance/implementation-index.md)
- [Ond for Kagura Machine Profile v1](../../targets/kagura-v1/machine.md)
