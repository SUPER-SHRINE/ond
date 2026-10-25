# Role構文と可視性
<!-- ond-spec-id: language/v1-roles -->

本章はOnd Language v1のroleによる宣言、import、文の選択と、名前解決・補完の文脈を定義する。\
各sourceの所属project、role集合と選択roleは[project role](../../build/roles.md)に従う。

## 構文
<!-- ond-spec-id: language/role-grammar -->

他章のproductionは[文法索引](./lexical.md#文法の定義元)を参照する。

```ebnf
RoleDecl      = "role" RoleNames RoleDeclBlock [ "else" RoleDeclBlock ] .
RoleDeclBlock = "{" { SourceItem ";" } "}" .
RoleStmt      = "role" RoleNames Block [ "else" Block ] .
RoleNames     = identifier { "," identifier } .
```

`role`はRoleDeclとRoleStmtの先頭位置で文脈keywordとなる。\
role名は少なくとも1個を要求し、末尾commaと同一リスト内の重複を認めない。\
各role名はそのsourceの所属projectのnamesの要素であることを成立条件とし、未知名はsource位置付きcompile errorとなる。

RoleDeclは[SourceItem](./names-and-scopes.md#構文)としてtop-levelに置き、RoleStmtは[Statement](./statements.md#構文)として関数内に置く。\
RoleDeclBlock内はimport、top-level宣言と入れ子のRoleDecl、RoleStmtの各block内は通常の文と入れ子のRoleStmtで構成する。\
package clauseはsource fileの先頭に1個置き、role内には置かない。\
`else`は対応する閉じ`}`との間にsemicolonを置かずに接続し、else省略時は選択対象を持たない空branchとして扱う。

## 対象集合と選択
<!-- ond-spec-id: language/role-selection -->

所属projectで宣言されたroleの集合をR、RoleNamesに列挙した集合をSとする。\
roleによる制限のない外側の対象集合はRである。\
親の対象集合をPとするとthen branchの実効対象集合はP∩S、else branchはP∩(R∖S)となる。\
複数名はORであり、入れ子の各段階で親との積集合を取る。\
この計算はelseも含めsourceの各宣言、importと文へ継承する。

選択role rが実効対象集合に含まれるbranchだけを選択する。\
選択したRoleDeclはその位置でbranch内のSourceItem列へ展開し、順序を保つ。\
選択したRoleStmtはそのbranchのblockを1個の文として実行し、選択されないbranchはeffectもruntime条件評価も持たない。\
return到達判定、break/continue、deferの位置と登録には、選択後の文列へ[通常の文規則](./statements.md)を適用する。\
role構文自体はloopまたはdeferの境界を追加しない。

実効対象集合が空のbranchはどのroleでも選択しない。\
そこでも次節の字句・構文・role名検査を行い、名前解決、補完候補、生成の対象は空とする。

## 検査範囲とimport
<!-- ond-spec-id: language/role-validation -->

通常/testのcompile modeと[package採用規則](../../build/packages.md#source探索境界)に従って収集したsourceの全branchを字句解析・構文解析し、role名とRoleNamesの成立条件を検査する。\
非選択branchの未知role名、構文errorと不正tokenもcompile errorとなる。\
通常の名前解決、型検査、定数検査と宣言重複検査は選択後のsourceに適用する。\
非選択部分の未解決名、型不一致、定数errorはそのcompileの失敗原因にしない。

各r∈Rについてfile-level role構文を展開したSourceItem列では、ImportDeclはすべてTopLevelDeclより前に現れることを構文成立条件とする。\
roleなしprojectでは通常のSourceItem列に同じ順序条件を適用する。\
この順序検査は各投影のtoken位置だけを比較し、非選択roleの名前解決やimport解決を実行しない。

import pathの解決、package到達性とimport循環は選択したImportDeclだけから求め、[package規則](./names-and-scopes.md#packageとimport)を適用する。\
非選択importはpackage到達性、alias衝突と依存build_idの直接import集合へ加えない。\
依存manifestの登録・lockの検証は[package build契約](../../build/packages.md)に従う。

## 名前解決とscope
<!-- ond-spec-id: language/role-visibility -->

source上でrole branchの内側にない位置を通常role文脈、内側にある位置をその実効対象集合Cのrole文脈とする。\
Rの全要素を指定したrole branchもrole文脈であり、通常role文脈と区別する。\
role内の関数、method、operatorとgeneric宣言の本体、signature、initializerは定義位置の文脈を継承し、call元や具体化要求元の文脈で置換しない。

同じproject内の名前解決では、既存のscope、公開/private、型parameterと通常/test可視性を満たす宣言のうち、次の条件を満たすものを候補とする。\
import aliasにも同じ条件を適用する。

| 参照位置 | 通常role文脈の宣言 | role内の宣言（実効対象集合D） |
| --- | --- | --- |
| 通常role文脈 | 候補に含む | 候補に含めない |
| C≠∅のrole文脈 | 候補に含む | C⊆Dの場合だけ候補に含む |
| C=∅のrole文脈 | 候補に含めない | 候補に含めない |

選択roleが1個でも、Cをそのsingletonへ狭めず上表を適用する。\
通常位置からrole限定宣言名を手書きした場合も候補に含まれず、他に解決先がなければcompile errorとなる。\
同じprojectのpackage-qualified参照にも上表を適用し、project境界を越えた公開APIは[project境界規則](../../build/roles.md#project境界)に従う。

RoleDeclの波括弧は新たなpackage scopeやfile scopeを導入しない。\
top-level宣言は元のpackage scope、import aliasは元のfile scopeに属する。\
RoleStmtのthen/elseはそれぞれ独立したblock scopeであり、そのlocal宣言はblock外へ漏れない。\
選択されたbranch内では外側のlocalを通常どおり参照・shadowできる。

同一scopeとnamespaceの同名宣言の重複禁止は、その実効対象集合が交差する宣言に適用する。\
通常宣言の対象集合はこの比較ではRとし、roleなしprojectでは通常の重複禁止を適用する。\
実効対象集合の交わらないbranch別同名宣言は許されるが、異なる宣言を複数roleにまたがる一つのbindingとして合成しない。\
各候補には単一宣言についてC⊆Dの成立を要求し、signatureや型が同じことを合成の根拠にしない。

## LSPの補完と診断
<!-- ond-spec-id: language/role-language-service -->

LSPはカーソル位置の通常/test文脈とrole文脈において、前節の名前解決で候補となる宣言を補完候補とする。\
通常sourceからtest-only宣言を隠す条件とrole可視性の条件を同時に満たす必要がある。\
role文脈内では通常宣言とCのすべてのroleで同一宣言として成立する候補を示し、通常role文脈ではrole限定宣言を示さない。\
local名、private名、member、genericの既存のscopeと型制約を保つ。

カーソル位置の解析では、現在のbuildの選択roleにかかわらず、その位置の実効対象集合C全体を使う。\
LSPは非選択branchについてもその位置の文脈で意味診断を返せるが、その診断は[compile時の検査範囲](#検査範囲とimport)を拡大しない。\
候補の存在しない空対象集合のbranchで、空集合の包含関係を根拠にすべての宣言を候補とすることは認めない。\
索引、候補、診断の再利用と設定変更時の失効は[role cache入力](../../build/roles.md#cache入力)に従う。
