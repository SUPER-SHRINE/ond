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

RAM baseと容量は呼出側が[link設定](../../build/packages.md#compileとlink設定)へ指定する。\
実行環境はRAM、MMIO map、unmapped領域を含むmemory mapと終了経路を定める。\
OndのKagura profileはその配置値と実行環境のmemory map・終了経路を使う。

## 3. RAM Layout
<!-- ond-spec-id: targets/kagura-v1/ram-layout -->

実行可能バイナリのsection、コピー範囲、heap_base、STACK_BOTTOM、STACK_TOPと初期SPは、[配置契約](../../artifacts.md#配置容量と出力範囲)に従う。\
heapはheap_baseから上方向へ、stackはSTACK_TOPから下方向へ成長する。\
stack予約範囲は`[STACK_BOTTOM, STACK_TOP)`であり、heap allocatorはSTACK_BOTTOM以上を割り当てない。

stack_sizeが4 mod 8の場合も予約範囲全体を保持し、8-byte単位のframeに使えない下端4 byteをheapへ転用しない。\
stack_size=4も配置の成立条件を満たせる。

sectionの保存、ロード、zero初期化は[実行可能バイナリ](../../artifacts.md#ロード完了時の状態)を定義元とする。\
配置と命令の個別検査でLinkPlan論理値を使う場合は、その検査に指定されたsection addressとruntime境界を入力とする。

### 3.1 Heap allocator
<!-- ond-spec-id: targets/kagura-v1/heap-allocator -->

heapは`heap_base`から`STACK_BOTTOM`までの領域をallocationへ提供する。\
allocationの共通保証は[言語のメモリ管理](../../language/v1/execution.md#メモリ管理)に従う。

allocation failure、size計算overflow、確保領域が`[heap_base, STACK_BOTTOM)`に収まらない要求は`nil`を返す。\
zero-sizeの`T`を正の個数確保するときも物理領域を予約し、言語上の型sizeはzeroを維持する。

allocationに用いるaddressの選択、探索順、header、block分割・結合、heap cursorの更新方式は実装裁量とする。

### ゼロサイズとaccessの境界
<!-- ond-spec-id: targets/kagura-v1/mir-boundary-decisions -->

外部から観測されるゼロサイズstorageとaggregate accessは以下の規則に従う。

### 3.2 ゼロサイズとaggregate access
<!-- ond-spec-id: targets/kagura-v1/mir-zero-sized-types -->

空structはsize 0、alignment 1、`[0]T`はsize 0、alignment `alignof(T)`とする。\
物理storageを要する場所ではnilと区別するため最低1 byteを予約し、型sizeは0を維持する。\
allocationの結果は[heap契約](#31-heap-allocator)に従う。

### 3.3 Ordered aggregate access
<!-- ond-spec-id: targets/kagura-v1/mir-ordered-aggregate -->

aggregateのaccess対象はstructのfieldとarrayのelementであり、struct宣言順、array index昇順に再帰してscalar accessを行う。\
aggregate copyはsource snapshotの読取完了後にdestinationへ書く。\
途中accessがfaultした場合は完了済みwriteとMMIO副作用を保持し、その場で実行を停止する。\
アクセス順序そのものを観測可能な契約として保持する。\
[非公開ローカル集約値](#34-非公開ローカル集約値)に限り、その節の条件でRAM accessを省略できる。

### 3.4 非公開ローカル集約値
<!-- ond-spec-id: targets/kagura-v1/private-local-aggregate -->

Kagura v1で、実行制御が途中から進入も分岐もしない単一の基本blockに扱いが収まり、次の条件を満たすstructまたは固定長配列のローカル変数とその一時値に適用する。

- 集約を再帰的に展開したscalar要素は1個以上4個以下で、配置sizeは16 byte以下とする
- block内のmemory accessはアドレスを取得しないローカルstorageと一時値に閉じ、対象の集約内の投影は宣言済みfieldまたは範囲内の定数indexで決まる
- block内の処理は呼出しを必要としない値演算、検査、ローカルの読書きとコピーで構成し、関数呼出境界ではscalarの引数と直接返却の範囲に収まる戻り値を扱う
- 対象の各要素は[配置契約](../../artifacts.md#配置容量と出力範囲)に従い、frame確保に成功したinvocationのRAM上で[型のalignment](#5-data-layout)を満たす
- 対象領域は型の幅で行う読書きが成功するRAMであり、そのaccessの副作用はRAMのバイト列の更新に限る

先行するblock内の書込みと入力値から要素の値を確定できる場合、その値を要素の読出しの結果として使える。\
書込先の要素と書込値が同一のバイト列を持つと確定できる場合、その書込みを省略できる。\
省略を含む実行でも、source snapshotの値はdestination更新前に確定し、各要素のzero value、`f32`のビット列と型同一性を保持する。\
比較元は、同じ操作、入力値、リンクで解決したアドレス値、frameと初期RAMを用いてaccessを省略せずに実行した結果とする。\
各操作の完了時とfault停止時には、その操作が使用したローカルstorage、値の一時領域とsnapshotの一時領域のバイト列、および完了済みの副作用が比較元と一致する。\
比較元で書き込まないpaddingのバイト列は保持する。\
frame内の配置、frame size、[stack overflow検査](./abi.md#7-stack-overflow)とその失敗時点も比較元と一致する。

## 4. Startup
<!-- ond-spec-id: targets/kagura-v1/startup -->

loaderは[責務の定義](../../build/packages.md#責務)に従ってsectionのロードとzero初期化、SPの設定を行い、linkされたruntime entryへ制御を渡す。\
実行可能バイナリのexternalとselfのloader、および制御移行時の状態は、[起動方式](../../artifacts.md#外部起動とセルフ起動)と[ロード契約](../../artifacts.md#ロード完了時の状態)に従う。\
entryは[言語の初期化順序](../../language/v1/execution.md#初期化とエントリ)に従ってinitializerを呼び、最後にmainを呼ぶ。\
mainから戻った際の制御経路は[link.exit](../../build/packages.md#compileとlink設定)に従う。

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
- f32は[言語のbinary32 bit pattern](../../language/v1/expressions.md#floating-point-arithmetic)をlittle-endianで配置する
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
- [operator解決](../../language/v1/expressions.md#user-defined-operator)でbuiltin raw pointer indexを選んだ場合、`*T`の`pointer[index]`の実効addressは`(pointer + index * sizeof(T)) mod 2^32`とする
  - `index` は `u32` とする
- raw indexed accessの有効address、allocation範囲とpointer生存期間は利用側の成立前提とする
- scalar の参照外しは型幅と同じ 1 回の bus access とする
- aggregate の読み書きは field または element 単位の複数 bus access とする
- aggregateのaccess順とsnapshot copyは[Ordered aggregate access](#33-ordered-aggregate-access)に従う
- fault時は[ordered aggregate access](#33-ordered-aggregate-access)の完了済みwrite/MMIO保持と停止規則に従う
  - destinationへのcopy開始はsource snapshot読取りの完了後である
- pointerによるbus accessの回数、幅、実行順を保持する
- 有効な RAM または MMIO address は、stack、heap、startup segment と重なっていてもその実体を参照する
  - 非公開ローカル集約値のaccessを一部省略するときのstack上のstorageは[非公開ローカル集約値](#34-非公開ローカル集約値)に従う
- alignment違反および機種bus上で許可されないaccessはfaultとする
  - nilの値は0だが、address 0の物理的なmappingは実行環境が定義する
- `.text`と`.rodata`への直接書込みは型検査errorとなる
  - raw pointer accessはbus mappingの物理的な読み書き規則に従う

## 7. Project Configuration
<!-- ond-spec-id: targets/kagura-v1/project-configuration -->

project、manifestとsource探索は[package build](../../build/packages.md#manifestとsource探索)を定義元とする。\
実行可能バイナリの永続設定は[compileとlink設定](../../build/packages.md#compileとlink設定)、配置は[実行可能バイナリ](../../artifacts.md#配置容量と出力範囲)に従う。

## Faultと観測結果
<!-- ond-spec-id: targets/kagura-v1/fault-observations -->

KaguraのCPU faultは[固定snapshot](./README.md#参照するkagura規範)のcodeとfaulting_pcを保持し、その場でCPUを停止してhostへ制御を返す。\
faulting_pcは実際にfaultした命令のPCである。\
debug metadataは次の意味分類を加え、CPU fault codeを変更しない。

| Ondの条件・観測操作 | Kagura CPU fault | debug operation |
| --- | --- | --- |
| builtin配列bounds check失敗 | INVALID_INSTRUCTION | BoundsCheck |
| runtime整数の除数zero | INVALID_INSTRUCTION | DivisionByZero |
| runtime signed minInt/-1 | INVALID_INSTRUCTION | SignedDivisionOverflow |
| runtime f32→整数の不正変換 | INVALID_INSTRUCTION | InvalidConversion |
| stack確保時の容量不足/underflow | INVALID_INSTRUCTION | StackOverflow |
| explicit trap | INVALID_INSTRUCTION | ExplicitTrap |
| raw load/storeのalignment違反 | UNALIGNED_ACCESS | MemoryRead/MemoryWrite |
| fetch/indirect jumpのPC alignment違反 | UNALIGNED_PC | 対応rangeがあればそのoperation |
| fetch/load/storeでbusがaccessを拒否 | BUS_FAULT | load/storeはMemoryRead/MemoryWrite、fetchは対応rangeのoperation |

nil function pointerのaddressは0であり、call結果は固定CPU規範と実行環境のaddress 0 mappingから定まる。\
例えばaddress 0がunmappedなら命令fetchがBUS_FAULTとなる。\
case表の`InvalidInstruction`、`BusFault`はそれぞれINVALID_INSTRUCTION、BUS_FAULTを表す。

faultした命令自身の副作用は固定CPU/bus規範に従い、その前に完了したwriteとMMIO effectは保持する。\
Ondの後続処理とdeferはそのfaultから実行を再開しない。
