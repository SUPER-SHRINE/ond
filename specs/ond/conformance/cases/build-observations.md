# Cacheとlinkの回帰入力
<!-- ond-spec-id: conformance/cases/build-observations -->

適用版はOnd Language v1、Kagura v1、artifact Contract revision 5とし、[版互換条件](../../artifacts.md#版識別と互換条件)を適用する。\
要件は`INIT-01`であり、操作の規範定義元は[cache](../../build/cache.md)、[compile/link](../../build/commands.md)、[linking](../../linking.md)、[初期化](../../language/v1/execution.md#初期化とエントリ)とする。

## Project入力と初期状態
<!-- ond-spec-id: conformance/build-observations/project-input -->

`ond.toml`は`[project]`と`name = "test"`の2行（LF区切り・末尾LFあり）とする。\
通常build、追加libraryなし、仮想sourceなし、既存cacheなしの状態から始め、同じcompiler executableを全操作で用いる。\
各sourceは次の内容であり、入力欄の改行は[共通復元規則](./README.md#入力と期待結果の読み方)に従う。

| File | 全source |
| --- | --- |
| `main.ond` | <code>package main<br>import &quot;test/right&quot;<br>import &quot;test/left&quot;<br>func main(){var _=left.Value()+right.Value()}</code> |
| `leaf/leaf.ond` | <code>package leaf<br>var Seed:i32=8<br>func Value()-&gt;i32{var n=Seed;return n/3}</code> |
| `left/left.ond` | <code>package left<br>import &quot;test/leaf&quot;<br>func Value()-&gt;i32{return leaf.Value()}</code> |
| `right/right.ond` | <code>package right<br>import &quot;test/leaf&quot;<br>func Value()-&gt;i32{return leaf.Value()}</code> |
| `other/other.ond` | <code>package other<br>func Value()-&gt;i32{return 5}</code> |

## 操作と比較
<!-- ond-spec-id: conformance/build-observations/operations -->

link設定はread_only=null、RAM=`0x1000:0x40000`、stack size=`0x10000`、main正常return先address=0とし、entry、heap base、stack bottomは[CLIのprogram指定](../../build/commands.md#ond-link)に従う。\
正常linkの観測はLinkedImageの全fieldとsection bytesの一致とする。

| 操作 | 期待 |
| --- | --- |
| cold compile後、manifestと参照artifactを読みlink | compile/link成功 |
| source・設定・compiler不変でwarm compileしlink | coldと同じLinkedImage |
| cold manifestのpackagesからtest/leafのArtifactRefを除去してlink | 依存入力不足のlink error、CLI終了code 1 |
| cold manifestのcontract.revisionを6へ変更してlink | 版不適合のlink error、CLI終了code 1 |
| cold manifestのcompiler_idをold-compilerへ変更してlink | 生成元不一致のlink error、CLI終了code 1 |
| leafのSeedだけ9へ変更して再compileし、cold manifestのleaf ArtifactRefだけを新しい参照へ置換してlink | 依存build_id不整合のlink error、CLI終了code 1 |

この入力での観測はcache再利用時のimageと依存整合性である。\
各packageのinitializerの存在と内部表現は[初期化の許容規則](../../language/v1/execution.md#初期化とエントリ)に従い、初期化順全体の区別に必要な観測はこのcaseの比較項目に含まれない。
