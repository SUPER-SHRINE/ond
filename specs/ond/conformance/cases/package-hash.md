# Package build_idの完全入力
<!-- ond-spec-id: conformance/cases/package-hash -->

要件は`HASH-01`、定義元は[公開識別子](../../build/cache.md#公開識別子)と[正規化集合](../../build/cache.md#意味型具体化定数の正規化)とする。適用Contractはrevision 6、target kagura-v1、abi ond-kagura-v1である。操作はsourceとmanifestからPackageBuildInputを構成してcanonical JSONとSHA-256を比較する。compiler_idはこのhash操作の論理入力として64個の`0`に固定し、mode=normal、package=main、dependencies={}とする。各caseは独立し、依存project、既存cacheは用いない。manifestの全UTF-8 bytesは`[project]`、`name = "main"`の2行、LF区切り・末尾LFありとする。source欄の`<br>`をLFへ復元し、末尾LFも含める。

## HASH-01.package-empty
<!-- ond-spec-id: conformance/package-hash/package-empty -->

Case IDは`HASH-01.package-empty`とする。`main.ond`全文:

<code>package main<br>func main(){}<br></code>

期待manifest_hash=`ad9dfebb3e1eab1840f379b08ba516d127330923ef130b8a458de00abcfbd92b`、source content_hash=`a774e3c1f96c0cb6e9acc84245f2cc3f7a03e298a4848f957bd0b4828d929631`。

期待PackageBuildInputのcanonical JSONは次の1行とし、hash対象はBOM・末尾LFのないUTF-8 bytesとする。

```json
{"compiler_id":"0000000000000000000000000000000000000000000000000000000000000000","contract":{"abi":"ond-kagura-v1","revision":6,"target":"kagura-v1"},"dependencies":{},"generation":{"constants":[],"instances":[],"interface":{"exports":{},"package":"main"},"nominals":[],"types":[{"Function":{"parameters":[],"returns":[]}}]},"manifest_hash":"ad9dfebb3e1eab1840f379b08ba516d127330923ef130b8a458de00abcfbd92b","mode":"normal","package":"main","sources":[{"content_hash":"a774e3c1f96c0cb6e9acc84245f2cc3f7a03e298a4848f957bd0b4828d929631","path":"main.ond"}]}
```

比較するbyte長は`556`、package build_idは`9548bc5baca3e9d935d46bfc1670ec4a7a444e5085ea6685a69ba54211149113`である。

## HASH-01.package-unused-body
<!-- ond-spec-id: conformance/package-hash/package-unused-body -->

Case IDは`HASH-01.package-unused-body`とする。`main.ond`全文:

<code>package main<br>func add(x:i32)-&gt;i32{return x+1}<br>func main(){}<br></code>

期待manifest_hash=`ad9dfebb3e1eab1840f379b08ba516d127330923ef130b8a458de00abcfbd92b`、source content_hash=`de4a8cdb3878cf02dd158dcd700021da1ddce93469eaacc9ffbbd94f2942ac2e`。

期待PackageBuildInputのcanonical JSONは次の1行とし、hash対象はBOM・末尾LFのないUTF-8 bytesとする。

```json
{"compiler_id":"0000000000000000000000000000000000000000000000000000000000000000","contract":{"abi":"ond-kagura-v1","revision":6,"target":"kagura-v1"},"dependencies":{},"generation":{"constants":[{"owner":{"declaration":{"Function":{"name":"add","package":"main"}},"receiver_arguments":[],"type_arguments":[]},"value":{"kind":{"Integer":1},"ty":"I32"}}],"instances":[],"interface":{"exports":{},"package":"main"},"nominals":[],"types":["I32",{"Function":{"parameters":["I32"],"returns":["I32"]}},{"Function":{"parameters":[],"returns":[]}}]},"manifest_hash":"ad9dfebb3e1eab1840f379b08ba516d127330923ef130b8a458de00abcfbd92b","mode":"normal","package":"main","sources":[{"content_hash":"de4a8cdb3878cf02dd158dcd700021da1ddce93469eaacc9ffbbd94f2942ac2e","path":"main.ond"}]}
```

比較するbyte長は`772`、package build_idは`1b05830c15bef6a316a98419ef2df018dc25ccc4d8c0da9cc003cb59ddf46430`である。

## 定数と型集合の境界
<!-- ond-spec-id: conformance/package-hash/constant-boundaries -->

次の各行は独立projectとし、`package main`、表の宣言、`func main(){}`をLF区切り・末尾LFありでmain.ondに置く。操作はCore受理後のGeneration抽出であり、各行のconstantsは表のownerと正規化値の集合に完全一致することを比較する。型構造とsource hashは変更したsourceから再構成する。

| Case ID | 宣言 | constantsのownerと値 |
| --- | --- | --- |
| HASH-01.maximal-constant | <code>func value(x:i32)-&gt;i32{return x+((1+2)*3)}</code> | Function main.value、I32 Integer9だけ |
| HASH-01.duplicate-constant | <code>func value(x:i32)-&gt;i32{if x&gt;0{return x+1};return x+1}</code> | Function main.value、I32 ZeroとInteger1。重複する1は1element |
| HASH-01.short-circuit | <code>const Flag=false &amp;&amp; (len([1]u8{256 as u8})==0)</code> | Constant main.Flag、Bool Zeroだけ。右枝の型はtypes抽出対象 |
| HASH-01.type-length | <code>type A [1+2]u8</code> | 空集合。typesはFunction([],[])、Defined(main.A,[])、Array(length3,elementU8)、U8の集合、nominalsはAのidentityと同Array underlying |
| HASH-01.nested-static | <code>const Grid=[2][1]u8{[1]u8{3},[1]u8{4}}</code> | Constant main.Grid。外側[2][1]u8のComposite値と、内側[1]u8の値3/4の3element。scalar3/4は各aggregate値に包含 |
| HASH-01.nested-string | <code>const Text=[1][2]u8{&quot;ab&quot;}</code> | Constant main.Text。外側[1][2]u8のComposite値と、文字列実体[2]u8のbytes 61 62の2element。byte scalarは各aggregate値に包含 |
