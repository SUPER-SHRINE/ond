# Ond Built-in Functions v1

本書は、`ond` compiler が v1 時点で組み込みとして扱う関数と、その周辺の特別構文をまとめる。

この文書では次の 2 種類を区別する。

- 組み込み関数: 名前解決の結果、通常の user-defined function ではなく compiler が特別扱いする呼び出し
- 組み込み構文: 関数呼び出しではないが、同じく compiler が特別扱いする式や構文

組み込み名は binding として予約され、同名の local / const / global / function は宣言できない。

## 1. 一覧

### 組み込み関数

| Name | Signature                                                         | Summary |
| --- |-------------------------------------------------------------------| --- |
| `alloc` | `alloc[T](count: u32) -> *T` | 型付きの未初期化 heap 領域を確保する |
| `free` | `free(pointer: *T)` 相当                                            | `alloc` / `new` が返した領域を解放する |
| `load32` | `load32(address: u32 \| pointer) -> u32`                          | 32-bit 値をメモリから読む |
| `store32` | `store32(address: u32                    \| pointer, value: u32)` | 32-bit 値をメモリへ書く |
| `thisFile` | `thisFile() -> *u8` | 現在の論理source pathを返す |
| `thisLine` | `thisLine() -> u32` | 現在のsource行番号を返す |

### 組み込み構文

| Name | Form | Summary |
| --- | --- | --- |
| `new` | `new(T)` | 型 `T` 1 個分の zero-initialized heap 領域を確保する |
| 配列長 `len` | `len([N]T)` | 配列長を compile-time constant `u32` として返す |
| layout | `sizeof(T)`, `alignof(T)` | 型の size / alignment を `u32` で返す |
| `trap` | `trap "reason"` | 理由をdebug metadataへ記録し、回復不能なCPU trapで停止する |

## 2. `alloc`

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

複数の同型要素を持つ連続領域の確保に使う。`alloc[u8]`とすればraw byte bufferとして使える。

## 3. `free`

```ond
free(ptr)
free(nil)
```

- 引数は 1 個だけ必要
- 返り値はない
- `alloc(...)` または `new(...)` が返した未解放の先頭 pointer を渡す
- `nil` を渡しても何もしない

未定義動作の詳細を隠したい場面でも、少なくとも二重解放や不正 pointer の扱いは仕様で保証されていない前提で使う。

## 4. `len`

### 4.1 配列に対する `len`

```ond
var size: u32 = len(buffer)
```

- `buffer: [N]T` に対して使える
- 結果は配列長 `N` を表す compile-time constant `u32`
- 文字列literalも`[N]u8`なので同じ規則を使う。例えば`len("日本語")`は`9`である
- operand は通常どおり一度評価する

これは関数呼び出しではなく、型情報から compiler が直接解決する。

配列以外では完全一致する `operator len` を呼び出す。一致する宣言がなければ compile error とする。

## 5. `load32`

```ond
var status: u32 = load32(DeviceStatusReg)
```

- 引数は 1 個だけ必要
- 引数型は `u32` または pointer
- 返り値は `u32`

MMIO register や raw memory を 32-bit 単位で読む用途を想定している。

## 6. `store32`

```ond
store32(DeviceCommandReg, DeviceCmdStart)
store32(DeviceDestinationReg, dest as u32)
```

- 引数は 2 個だけ必要
- 第 1 引数は `u32` または pointer
- 第 2 引数は `u32`
- 返り値はない

MMIO register や raw memory を 32-bit 単位で書く用途を想定している。

## 7. `new`

```ond
var ptr: *[4]u8 = new([4]u8)
```

- `new(T)` は関数ではなく専用構文
- 返り値は `*T`
- `new([4]u8)` の返り値は `*[4]u8` であり、`*u8` への暗黙変換は行わない
- `T` 1 個分の領域を確保する
- zero value で初期化する
- allocation failure 時は `nil`

`alloc[T]`は未初期化の`T`を複数個格納する領域、`new(T)`はzero初期化した`T`を1個格納する領域の確保に使う。

## 8. `thisFile`

```ond
report(thisFile(), thisLine())
```

- 引数は取らない
- `thisFile()` 自身を含むsource fileの論理pathを、null終端された読み取り専用byte列への`*u8`として返す
- project root packageのfileはfile名だけを使い、それ以外はpackageのlogical import pathとfile名を`/`で連結する
- host上のproject root、library root、絶対pathは含めない
- path separatorはhost OSによらず`/`とする
- 指し示すstorageの寿命はプログラム全体とし、書き込んではならない
- 同じ論理pathのstorageは同一Object内で共有してよい

例えばproject rootの`main.ond`では`"main.ond\000"`相当、package `graphics/draw`の`render.ond`では`"graphics/draw/render.ond\000"`相当のstorageを指す。

## 9. `thisLine`

- 引数は取らない
- `thisLine()` 自身の先頭tokenが置かれた、1始まりのsource行番号を`u32`で返す
- 外側のcallや、現在の関数を呼び出した箇所の行番号には遡らない

## 10. `trap`

```ond
trap "player index is out of range"
trap `raw reason`
```

- `trap` は関数ではなく、戻らない文である。
- operand はquoted string literalまたはraw string literalを1個だけ取る。変数、定数、`[N]u8`値は渡せない。
- literalはUTF-8として有効でなければならない。escapeは通常の文字列literalと同じ規則でdecodeする。
- 理由はruntime valueへ変換せず、Object内のdebug metadataへ格納する。文字列用のread-only storageやpointerは生成しない。
- linkerは理由を`.onddebug`へ移し、実行命令は引数なしのCPU trapとする。
- `trap()` は存在しない。

## 11. 名前解決上の注意

組み込み名は lexical binding として宣言できない。field 名など binding を作らない member 名には使用できる。

## 12. よくある compile error

- `alloc(...)` は引数 1 個のみ
- `free(...)` は引数 1 個のみ
- `len(...)` は引数 1 個のみ
- `len(...)` の引数は配列のみ
- `load32(...)` は引数 1 個のみ
- `load32(...)` の address は `u32` または pointer
- `store32(...)` は引数 2 個のみ
- `store32(...)` の value は `u32`
- `thisFile()` と `thisLine()` は引数を取らない
- `trap` の直後にはUTF-8として有効なstring literalを置く

## 13. 関連仕様

- [Ond Language v1](./language.md)
- [Ond Debug Metadata v1](../debug-metadata.md)
- [Ond Compiler IR](../../../crates/compiler/docs/ond-ir.md)
- [Ond for Kagura Machine Profile v1](../targets/kagura-v1/machine.md)
