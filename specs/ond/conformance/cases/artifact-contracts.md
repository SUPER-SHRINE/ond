# 成果物・build・linkの契約case
<!-- ond-spec-id: conformance/cases/artifact-contracts -->

適用版はOnd v1、Kagura v1、artifact Contract revision 6とする。\
各caseは他caseから独立した入力を持ち、byte列を明記した行はBOM/末尾LFなしとする。\
操作が型のwire読出しである場合は、その型のschemaと値条件を比較し、link・source実行の追加成功を意味しない。\
error行は操作の失敗、link行は[CLI失敗契約](../../build/commands.md#入出力と失敗時)のlink errorを比較する。

## 閉schemaと値表現
<!-- ond-spec-id: conformance/artifact-contracts/wire -->

要件は`ART-01`、定義元は[wire](../../artifacts.md#json-wire形式)と[artifact型表](../../artifacts.md#packageartifactとcompilationmanifest)とする。\
共有入力Aは次のPackageArtifactの全JSON bytesである。

```json
{"build_id":"1111111111111111111111111111111111111111111111111111111111111111","compiler_id":"0000000000000000000000000000000000000000000000000000000000000000","contract":{"abi":"ond-kagura-v1","revision":6,"target":"kagura-v1"},"debug":{"functions":[],"package":"sample","ranges":[]},"dependencies":{},"imports":[],"initializer":null,"interface":{"exports":{},"package":"sample","types":[]},"object":{"name":"diagnostic","relocations":[],"sections":[],"symbols":[]},"package":"sample"}
```

| Case ID | 操作・入力 | 期待 |
| --- | --- | --- |
| ART-01.closed.accept | AをPackageArtifactとして読む | 受理 |
| ART-01.required-null | Aからinitializer fieldを削除して読む | 必須field欠落のerror |
| ART-01.unknown-nested | A.objectへextra:0を追加して読む | 未知fieldのerror |
| ART-01.tests-null | Aへtests:nullを追加して読む | tests型のerror |
| ART-01.duplicate-decoded-key | Contractとして <code>{"revision":6,"\u0072evision":6,"target":"kagura-v1","abi":"ond-kagura-v1"}</code> を読む | decode後の重複keyのerror |
| ART-01.composite-tuple | ValueKindとして <code>{"Composite":[[0,{"ty":0,"kind":"Zero"}]]}</code> を読む | tuple長2として受理（Value型参照の所属検査は別操作） |
| ART-01.composite-arity | ValueKindとして <code>{"Composite":[[0]]}</code> を読む | tuple長違反のerror |
| ART-01.enum-case | SectionKindとして <code>"text"</code> を読む | variant綴り違反のerror |
| ART-01.u32-overflow | u32として <code>4294967296</code> を読む | 範囲error |
| ART-01.integer-spelling | u32として <code>1.0</code>、<code>1e0</code>、<code>-0</code> を別々に読む | 各々整数表記error |
| ART-01.base64 | bytesとして <code>""</code>、<code>"TQ=="</code>、<code>"TWE="</code>、<code>"TWFu"</code> を別々に読む | hex bytesが順に空、4d、4d61、4d616e |
| ART-01.base64-reject | bytesとして <code>"TQ"</code>、<code>"TR=="</code>、<code>"TQ==\n"</code> を別々に読む | padding、pad bits、whitespaceの各error |

## Canonical JSONとhash
<!-- ond-spec-id: conformance/artifact-contracts/hashes -->

要件は`HASH-01`、定義元は[canonical JSON](../../build/cache.md#canonical-json)と[公開識別子](../../build/cache.md#公開識別子)とする。\
各入力値をcanonical化し、得たbytesとSHA-256の小文字hexを比較する。

| Case ID | 入力値 | canonical UTF-8 hex | SHA-256 |
| --- | --- | --- | --- |
| HASH-01.empty-object | 空object | 7b7d | 44136fa355b3678a1146ad16f7e8649e94fb4fc21fe77e8310c060f61caaff8a |
| HASH-01.string-escapes | a値=BS,TAB,LF,FF,CR,NUL,U+001F,quote,backslash,slashの順のstring、z値=u64最大、あ値=éのobject | 7b2261223a225c625c745c6e5c665c725c75303030305c75303031665c225c5c2f222c227a223a31383434363734343037333730393535313631352c22e38182223a22c3a9227d | 4de8ae025c8158e7530835967808cff223b8a64ce8126d19d1898c201256ad36 |
| HASH-01.scalar-key-order | key U+E000→0、key U+1F600→1のobject | 7b22ee8080223a302c22f09f9880223a317d | a712a1804e0980b85da1fbebfb21070144773d51ac011435af287d757b3e7dc5 |

`HASH-01.file-bytes`はAをBOM/末尾LFなしで保存した486byteと、Aの末尾にLFを1byte追加した487byteを別fileとする。\
content_hashは順に`da820c7d7eb015cba9c5f5d15858d6e6760316a977b9d4c8a1eb370170e5294a`、`3304a08fe5acfa8a103ff632a16fe71e1c8f9ad59e7c10203a19e402a7239ee6`と比較する。\
JSON値の一致を理由に同じbytesへ再encodeして比較しない。

## Manifest pathとmode
<!-- ond-spec-id: conformance/artifact-contracts/manifest -->

要件は`ART-01`、定義元は[参照整合](../../artifacts.md#manifest読出しと参照整合)とする。\
path解決操作の初期cwdを`/work/run`、file manifestを`/work/build/app.ondbuild`とする。\
Windows runnerはこの絶対directory構造を同じ相対関係の専用rootへ対応づける。

| Case ID | 入力 | 期待解決先 |
| --- | --- | --- |
| ART-01.path-file | file manifest、ArtifactRef.path=cache/a.ondpkg | /work/build/cache/a.ondpkg |
| ART-01.path-stdin | stdin manifest、ArtifactRef.path=cache/a.ondpkg | /work/run/cache/a.ondpkg |
| ART-01.path-absolute | いずれのmanifestでもArtifactRef.path=/work/shared/a.ondpkg | /work/shared/a.ondpkg |

modeと通常/testの起動整合は[test build](../../build/test-build.md)と[cache/link入力](./build-observations.md)も比較する。

test関数の成立条件は[test source規則](../../build/test-build.md#sourceと文脈)をcompileで比較し、一覧の入力整合は[manifest読出し](../../artifacts.md#manifest読出しと参照整合)をlinkで比較する。\
次の各入力は独立projectであり、manifestは`[project]`と`name = "test"`の2行、`main.ond`は`package main`と`func main(){}`の2行とする。\
`main_test.ond`は`package main`、`type Device struct{}`、表の宣言の3行とし、全fileはLF区切り・末尾LFあり、manifest記載以外の入力なし・既存cacheなし、操作はrevision 6でのtest compileとする。

| 入力宣言 | compile期待 |
| --- | --- |
| `func TestBad(x:u32){}` | compile error |
| `func TestBad()->u32{return 1}` | compile error |
| `func TestBad[T](){}` | compile error |
| `func(x:Device) TestBad(){}` | compile error |
| `func TestRoot(){}` | 成功。manifestとroot artifactのcase列はpackage=`test`、name=`TestRoot`、symbol=`main.TestRoot`の1件。test-only宣言は公開interfaceへ追加しない |

正常入力の`main.ond`に`func TestOrdinary(x:u32){}`を追加した場合も、case列はTestRootの1件とする。\
linkを含む操作ではrootのond.tomlへ[cache/link入力](./build-observations.md#project入力と初期状態)のlink・link.ram・link.exit tableを追記してからcompileする。\
正常入力に`left/left.ond`（`package left`、`func Value(){}`の2行）を追加してtest compileし、同じcompiler executableでrootをcwdとして`ond link target/test.ondbuild`を行うと成功する。\
各操作はこのcompile出力の独立したcopyへ適用し、変更artifactの元bytesからcontent_hashを再計算してArtifactRefも更新する。\
root artifactのcaseを空にして同caseを`test/left` artifactへ移した入力、およびroot artifactのcaseを同artifact内で複製した入力は、manifestのcase列を維持したままlinkし、それぞれ所属不一致と重複のlink errorを比較する。

## LinkPlan、symbol、短距離patch
<!-- ond-spec-id: conformance/artifact-contracts/linking -->

要件は`LINK-01`、定義元は[linking](../../linking.md)と[symbol集合](../../artifacts.md#linkplanとlinkedimage)とする。\
共有LinkPlan Lの論理値を次の記法で表す。\
この節は配置済みTextの先頭addressを4096に指定したsymbol・patch計算の個別検査であり、ONDXの生成やLinkPlanのJSON wire読出しを操作へ含めない。

```json
{"read_only":null,"memory_base":4096,"memory_size":65536,"stack_size":8192,"entry_symbol":"main.main","heap_base_symbol":"heap","stack_bottom_symbol":"bottom"}
```

共有Object Oはname="input"、Text section（alignment=4、memory_size=12、dataのhexは後述）、symbol列は次の2定義、relocation列はcase指定の1件とする。

```json
[{"name":"main.main","binding":"Exported","kind":"Function","section":"Text","offset":0,"size":4},{"name":"target","binding":"Exported","kind":"Function","section":"Text","offset":8,"size":4}]
```

| Case ID | O.Textのdata hexとrelocation | 期待 |
| --- | --- | --- |
| LINK-01.call16 | 34 12 00 cf 00 00 00 00 00 00 00 00。relocation={section:Text,offset:0,kind:Call16,target:target,addend:0} | site address4096、target4104、d=1、patch後先頭4byte=01 00 00 cf |
| LINK-01.jump16 | aa aa 20 c0 00 00 00 00 00 00 00 00。同relocationのkindだけJump16 | patch後先頭4byte=01 00 20 c0 |
| LINK-01.call-opcode-reject | 34 12 00 bf 00 00 00 00 00 00 00 00。Call16 relocation | JZ入力形違反のlink error |
| LINK-01.ram-end-align | call16のO、L.memory_sizeだけ65540 | RAM_ENDの8-byte alignment違反でlink error |
| LINK-01.four-byte-stack | call16のO、L.stack_sizeだけ4 | link成功、stack_top69632、stack_bottom69628、予約全4byteを保持 |

Oのrelocationを空、Text dataを12byteのzero、追加symbolを`{name:hidden,binding:Local,kind:Object,section:Text,offset:4,size:4}`とした`LINK-01.local-hidden`はlink成功し、symbolsが`{main.main:4096,target:4104,heap:4108,bottom:61440}`だけであることを比較する。\
この操作はlinkだけであり、machine起動とstackframe実行を行わない。

## Git lock v2
<!-- ond-spec-id: conformance/artifact-contracts/git-lock -->

要件は`DEP-01`、定義元は[lock v2](../../build/packages.md#git依存lock-v2)とする。\
lock読出しの共有入力Kは次の全文である。

```toml
version = 2
[[package]]
owner = "app"
name = "math_dep"
git = "https://example.invalid/math.git"
rev = "release"
commit = "0123456789012345678901234567890123456789"
```

| Case ID | 操作・初期状態 | 期待 |
| --- | --- | --- |
| DEP-01.schema | Kをlock schemaとして読む | 受理。例示commitの実在をこの操作では要求しない |
| DEP-01.zero-edges | version=2とpackage=[]だけのlockを読む | Git edge0件の受理 |
| DEP-01.identity-reuse | 必要Git edge=(app,math_dep,上記URL,rev release)とKの要求同一性を照合 | lock entryの対応と要求一致を受理 |
| DEP-01.rev-change | 同じ初期状態で要求revだけnextへ変更しKを照合 | 要求不一致error |
| DEP-01.missing-edge | 同じ必要edgeに対し空package列を照合 | 必要entry欠落error |
| DEP-01.extra-edge | 必要Git edge0件に対しKを照合 | 余剰entry error |
| DEP-01.version-one | K.versionだけ1へ変更しschema読出し | version不一致error |

これらの操作はschemaと必要edgeに対する要求同一性の照合を対象とし、repositoryのcommit読取りはこのcase群の操作へ含めない。\
commitの存在・解決可能性の検査は[lock本文](../../build/packages.md#git依存lock-v2)の規範に従う。
