# Ond for Kagura Target Runtime Contract

本書はOndのKagura backendが用いる型配置・runtimeの契約を定める。具体的なbus配置・ROM形式・起動は機種SDKが所有する。関数呼出規約は[ABI](./abi.md)を参照。

## 1. Machine Profile

- word size: 32 bit
- address size: 32 bit
- byte addressed
- little-endian
- RAM と MMIO は同じ flat bus address space を共有する。
- CPU は `ADD`, `NAND`, `SHIFT`, `MUL`, `LDB`, `LDH`, `LDW`, `STB`, `STH`, `STW`, `JZ`, `CMP` を実装する。
- CPU fault は machine を停止して host へ返す。

## 2. Address Space

RAM base・容量・MMIO map・unmapped領域は機種SDKが定義し、LinkPlanに渡す。OndのKagura backendは固定の機種mapや終了deviceを要求しない。EnbuではEnbu SDKがROM window、RAM、BANK registerを管理する。

## 3. RAM Layout

RAM 内は低 address から次の順に配置する。

```text
.text | .rodata | .data | .bss | heap ... free ... stack
                                          ->       <-
```

- startup segment は `.text`, `.rodata`, `.data`, `.bss` の順に置く。
- segment 間には alignment padding だけを置ける。
- `heap_base = align_up(end_of_bss, 4)` とし、heap は上方向へ成長する。
- `RAM_END = RAM_BASE + ram_size`
- `STACK_TOP = RAM_END`
- `STACK_BOTTOM = STACK_TOP - stack_size`
- stack は `STACK_TOP` から下方向へ成長する。
- startup segment の末尾は `STACK_BOTTOM` 以下でなければならない。
- heap allocator は `STACK_BOTTOM` 以上を割り当ててはならない。

sectionの保存形式・load・zero初期化は機種SDKが定義する。

### 3.1 Heap allocator

- heap は `heap_base` から `heap_cursor` までの連続した block 列として管理する。
- 各 block は 4-byte header と 4-byte alignment へ切り上げた payload からなる。header は block 全体の size と使用中 flag を保持する。
- `alloc` は block 列を低 address から線形探索する first-fit とする。
- 十分に大きい free block は、残りが header と最小 payload を保持できる場合に分割する。
- 適切な free block がなければ `heap_cursor` から新しい block を確保する。size 計算の overflow または `STACK_BOTTOM` 超過時は `nil` を返す。
- `free` は block を free にした後、物理的に隣接する前後の free block と結合する。末尾の free block は block 列から除き、`heap_cursor` を巻き戻す。
- block の移動による compaction は行わない。

header は allocator の内部表現であり、Ond program へ返す pointer は payload の先頭を指す。

`alloc(0)`はnilを返す。`new(T)`は型のsizeに必要な領域を確保し、成功時にzero化する。size=0の型でも最低payload（現allocatorでは4-byte alignmentへ切り上げた領域）を確保し、成功時は非nil、確保失敗時はnilとする。型のsizeは0のままである。

## 4. Startup

機種SDKがsectionのロードとzero初期化、SPの設定を行い、linkされたruntime entryへ制御を渡す。entryはMIRのpackage初期化順序に従ってinitializerを呼び、最後にmainを呼ぶ。mainから戻った際の処理はSDKが指定する。EnbuではROM内loaderが起動を行い、正常return後はRAM上のloopに入る。

## 5. Data Layout

| Type | Size | Alignment |
| --- | ---: | ---: |
| `u8`, `i8`, `bool` | 1 | 1 |
| `u16`, `i16` | 2 | 2 |
| `u32`, `i32`, `f32`, `*T` | 4 | 4 |
| interface | 8 | 4 |

- guest type の最大 alignment は 4 byte とする。
- `bool` は memory 上の `0` を false、非 `0` を true とする。compiler は `0` または `1` を store する。
- `f32` は IEEE 754 binary32 の bit pattern を little-endian で配置する。
- `*T` は 32-bit bus address で、`nil` は `0x00000000` とする。
- interfaceはoffset 0にdata pointer、offset 4にvtable pointerを持つ。zero valueは両方が`0x00000000`である。
- `[N]T` は header のない連続した `N` 要素とし、alignment は `alignof(T)`、size は `N * sizeof(T)` とする。
- struct field は宣言順に置き、各 field をその alignment へ揃える。struct の alignment は field の最大値、size はその alignment の倍数へ切り上げる。
- 空structはsize=0、alignment=1とする。`[0]T`はsize=0、alignment=`alignof(T)`とする。address取得用slot・Indirect引数temporary等のゼロサイズstorageは最低1 byteを予約してnilと区別し、型のsizeは0を維持する。異なるゼロサイズ値のアドレス同一性について追加保証はしない。ゼロサイズheap allocationは§3.1に従う。
- padding byte の値は観測可能な意味を持たない。

全型の zero value は全 byte が `0` の表現とする。

## 6. Raw Pointer Access

- pointer の生成は memory を確保または予約しない。
- `*T` の `pointer[index]` が使う実効 address は `(pointer + index * sizeof(T)) mod 2^32` とする。`index` は `u32` とする。
- indexed access は base pointer、allocation の範囲、pointer の寿命を検査しない。
- scalar の参照外しは型幅と同じ 1 回の bus access とする。
- aggregate の読み書きは field または element 単位の複数 bus access とする。
- aggregate内部はstructの宣言順、arrayのindex昇順に再帰的にscalar accessへ展開し、paddingへaccessしない。aggregate copyではsourceのsnapshotを完成させてからdestinationへこの順で書く。
- aggregate全体のatomicityは保証しない。途中のaccessでfaultした場合、それ以前に完了した書き込みやMMIOの副作用は巻き戻さない。source読み取り中のfaultではdestinationへの書き込みを開始しない。これはaccessの並べ替えを許可する規則ではない。
- pointer による bus access は削除、統合、複製、順序変更しない。
- 有効な RAM または MMIO address は、stack、heap、startup segment と重なっていてもその実体を参照する。
- alignment違反および機種bus上で許可されないaccessはfaultとする。nilの値は0だが、address 0の物理的なmappingは機種SDKが定義する。
- `.text` と `.rodata` は Ond の型検査上は書き込み不可だが、RAM の write protection は行わない。

## 7. Project Configuration

`ond.toml`があるdirectoryをproject rootとし、root自体がmain packageとなる。manifestは依存関係と
source探索境界を定義する。RAM配置とstack予約量はmachine SDKまたは`ond link`の`LinkPlan`が決定し、
Ond project manifestには保持しない。

`[project].name`は依存先projectのimport名前空間として使用する。依存されるprojectではOndの
identifierとして有効な名前を必須とする。それ以外のproject metadataと一覧表示は将来拡張とし、
v1のコンパイル結果には影響しない。
