# Ond for Kagura Target Runtime Contract
<!-- ond-spec-id: targets/kagura-v1/runtime-contract -->

本書はOndのKagura backendが用いる型配置・runtimeの契約を定める。\
具体的なbus配置・ROM形式・起動を担う役割は[責務の定義](../../build/packages.md#責務)に従い、対象実行環境とそのloader・配布ツールが構成する。\
関数呼出規約は[ABI](./abi.md)を参照する。

## 1. Machine Profile
<!-- ond-spec-id: targets/kagura-v1/machine-profile -->

- word size: 32 bit
- address size: 32 bit
- byte addressed
- little-endian
- RAM と MMIO は同じ flat bus address space を共有する
- CPU は `ADD`, `NAND`, `SHIFT`, `MUL`, `LDB`, `LDH`, `LDW`, `STB`, `STH`, `STW`, `JZ`, `CMP` を実装する
- CPU fault は machine を停止して host へ返す

## 2. Address Space
<!-- ond-spec-id: targets/kagura-v1/address-space -->

RAM baseと容量は呼出側が[LinkPlan](../../artifacts.md#linkplanとlinkedimage)へ指定する。\
実行環境はRAM、MMIO map、unmapped領域を含むmemory mapと終了経路を定める。\
OndのKagura profileはLinkPlanの配置値と実行環境のmemory map・終了経路を使う。

## 3. RAM Layout
<!-- ond-spec-id: targets/kagura-v1/ram-layout -->

RAM 内は低 address から次の順に配置する。

```text
.text | .rodata | .data | .bss | heap ... free ... stack
                                          ->       <-
```

- startup segment は `.text`, `.rodata`, `.data`, `.bss` の順に置く
- segment 間には alignment padding だけを置ける
- `heap_base = align_up(end_of_bss, 4)` とし、heap は上方向へ成長する
- `RAM_END = RAM_BASE + ram_size`
- `STACK_TOP = RAM_END`
- `STACK_BOTTOM = STACK_TOP - stack_size`
- stack は `STACK_TOP` から下方向へ成長する
- startup segment の末尾は `STACK_BOTTOM` 以下でなければならない
- heap allocator は `STACK_BOTTOM` 以上を割り当ててはならない

sectionの保存形式は配布形式が定め、loaderはsectionをloadしてinitialized dataの末尾からmemory_sizeまでをzero初期化する。

### 3.1 Heap allocator
<!-- ond-spec-id: targets/kagura-v1/heap-allocator -->

heapは`heap_base`から`STACK_BOTTOM`までの領域をallocationへ提供する。\
成功したallocationは要求されたsizeを格納できる連続領域と`alignof(T)` alignmentを持ち、存続中のallocation同士は重ならない。\
returned pointerは利用可能な領域の先頭を指す。\
領域移動を伴わないため、allocationが存続する間pointerは同じ領域を示す。

`alloc[T](count)`は`count`個の`T`を格納する領域を確保し、payloadの初期byte値は実装裁量とする。\
`new(T)`は`T`一個分を確保し、成功時にzero valueで初期化する。\
allocation failure、size計算overflow、確保領域が`[heap_base, STACK_BOTTOM)`に収まらない要求は`nil`を返す。\
`alloc[T](0)`は`nil`を返す。\
zero-sizeの`T`に対する`new(T)`とpositive countの`alloc[T](count)`は物理領域を確保し、成功時はnon-nilを返し、言語上の型sizeはzeroを維持する。

`free`は`new`または`alloc`から得た未解放のallocation先頭pointerと`nil`を受け取る。\
allocationを解放し、`free(nil)`はallocation状態を保持して正常に完了する。\
allocationの生存期間、alignment、size、分離、初期化値が外部契約となる。\
allocatorの探索順、header、block分割・結合、heap cursorの更新方式は実装sourceの責務とする。

### ゼロサイズとaccessの境界
<!-- ond-spec-id: targets/kagura-v1/mir-boundary-decisions -->

外部から観測されるゼロサイズstorageとaggregate accessは以下の規則に従う。

### 3.2 ゼロサイズとaggregate access
<!-- ond-spec-id: targets/kagura-v1/mir-zero-sized-types -->

空structはsize 0、alignment 1、`[0]T`はsize 0、alignment `alignof(T)`とする。\
物理storageを要する場所ではnilと区別するため最低1 byteを予約し、型sizeは0を維持する。\
newによるzero-size allocationは成功時non-nil、`alloc(0)`はnilとなる。

### 3.3 Ordered aggregate access
<!-- ond-spec-id: targets/kagura-v1/mir-ordered-aggregate -->

aggregateのaccess対象はstructのfieldとarrayのelementであり、struct宣言順、array index昇順に再帰してscalar accessを行う。\
aggregate copyはsource snapshotの読取完了後にdestinationへ書く。\
途中accessがfaultした場合は完了済みwriteとMMIO副作用を保持し、その場で実行を停止する。\
アクセス順序そのものを観測可能な契約として保持する。

## 4. Startup
<!-- ond-spec-id: targets/kagura-v1/startup -->

loaderは[責務の定義](../../build/packages.md#責務)に従ってsectionのロードとzero初期化、SPの設定を行い、linkされたruntime entryへ制御を渡す。\
entryは[言語の初期化順序](../../language/v1/execution.md#初期化とエントリ)に従ってinitializerを呼び、最後にmainを呼ぶ。\
mainから戻った際の処理は呼出側が指定したtermination instructionsが定める。

上記は通常buildのstartup契約である。\
test buildでもloaderによるload・zero初期化・SP設定の境界を保ち、詳細は[テストビルド仕様](../../build/test-build.md)に従う。

## 5. Data Layout
<!-- ond-spec-id: targets/kagura-v1/data-layout -->

| Type | Size | Alignment |
| --- | ---: | ---: |
| `u8`, `i8`, `bool` | 1 | 1 |
| `u16`, `i16` | 2 | 2 |
| `u32`, `i32`, `f32`, `*T`, function pointer | 4 | 4 |

- guest type の最大 alignment は 4 byte とする
- `bool` は memory 上の `0` を false、非 `0` を true とする
  - compiler は `0` または `1` を store する
- `f32` は IEEE 754 binary32 の bit pattern を little-endian で配置する
- `*T` は 32-bit bus address で、`nil` は `0x00000000` とする
- `[N]T` は header のない連続した `N` 要素とし、alignment は `alignof(T)`、size は `N * sizeof(T)` とする
- 固定長配列の `N * sizeof(T)` が `2^32-1` byte を超える場合は compile error とする
  - 積は数学的な整数として判定し、32 bit への切り詰めや折り返しを行わない
  - 推論長配列も長さの確定後に同じ上限を適用する
  - 要素が defined type、struct、配列の場合も、padding を含む要素型の size を使う
  - 未使用の型宣言、pointer の参照先、generic の具体化結果も検査する
  - `N = 0` または `sizeof(T) = 0` の積は `0` とし、入れ子に含まれる各配列型もそれぞれ検査する
- struct field は宣言順に置き、各 field をその alignment へ揃える
  - struct の alignment は field の最大値、size はその alignment の倍数へ切り上げる
- zero-size型のsize/alignment、addressable storageとheap結果は[ゼロサイズの境界](#32-ゼロサイズとaggregate-access)に従う
- padding byte の値は観測可能な意味を持たない

全型の zero value は全 byte が `0` の表現とする。

例えば `[4294967295]u8` の size は上限内だが、`[1073741824]u32` の size は `2^32` byte なので型として拒否する。\
上限内の配列型であっても、実際の storage 配置や heap 確保が成功するとは限らない。

## 6. Raw Pointer Access
<!-- ond-spec-id: targets/kagura-v1/raw-pointer-access -->

- pointerはaddress値であり、対応storageは変数宣言または明示allocationによって確保する
- `*T` の `pointer[index]` が使う実効 address は `(pointer + index * sizeof(T)) mod 2^32` とする
  - `index` は `u32` とする
- raw indexed accessの有効address、allocation範囲とpointer生存期間は利用側の成立前提とする
- scalar の参照外しは型幅と同じ 1 回の bus access とする
- aggregate の読み書きは field または element 単位の複数 bus access とする
- aggregateのaccess順とsnapshot copyは[Ordered aggregate access](#33-ordered-aggregate-access)に従う
- fault時は[ordered aggregate access](#33-ordered-aggregate-access)の完了済みwrite/MMIO保持と停止規則に従う
  - destinationへのcopy開始はsource snapshot読取りの完了後である
- pointerによるbus accessの回数、幅、実行順を保持する
- 有効な RAM または MMIO address は、stack、heap、startup segment と重なっていてもその実体を参照する
- alignment違反および機種bus上で許可されないaccessはfaultとする
  - nilの値は0だが、address 0の物理的なmappingは実行環境が定義する
- `.text`と`.rodata`への直接書込みは型検査errorとなる
  - raw pointer accessはbus mappingの物理的な読み書き規則に従う

## 7. Project Configuration
<!-- ond-spec-id: targets/kagura-v1/project-configuration -->

`ond.toml`があるdirectoryをproject rootとし、実行projectのrootがmain packageとなる。\
manifestは依存関係と\
source探索境界を定義する。\
RAM配置とstack予約量は呼出側が`ond link`の`LinkPlan`で決定し、\
project manifestは依存関係とsource探索の入力として用いる。

`[project].name`はproject自身と依存先projectのimport名前空間として使用する。\
すべてのprojectでOndの\
identifierとして有効な名前を必須とする。\
project name以外のproject metadataおよび一覧表示は\
sourceのコンパイル結果はこの名前規則と依存/source入力により定まる。
