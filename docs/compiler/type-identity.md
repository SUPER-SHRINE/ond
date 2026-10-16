# 具体化したgeneric型とsymbolの内部同一性

この文書はcompilerの実装説明であり、言語仕様や成果物のschemaを定義しない。\
型同一性の規則は[言語仕様](../../specs/ond/v1/language.md#型同一性)、公開情報とcacheの契約は[package build仕様](../../specs/ond/package-build.md)を参照する。

## 型名の生成

`crates/compiler-core/src/pipeline/hir/identity.rs`は、generic型の宣言名と順序付きの型argument列から具体化名を生成する。\
project内の割当順で変わる`TypeId`や`PackageId`は名前へ含めず、公開情報へ保存する前から同じ名前で検索・予約・再利用する。

内部encodingは型種別のtag、UTF-8のbyte長を伴う文字列、要素数を伴う列で構成される。\
pointerは参照先、配列は長さと要素型、関数はparameter型列と戻り値型列を順に記録する。\
無名structはfieldの順序・名前・型に加えて、非公開fieldのownerの論理package pathを保持する。\
source位置と関数parameter名はencodingに関与しない。\
具体化名の外側にある宣言名全体にもbyte長を付け、package path中の`[`や`:`がargument encodingとの境界を移動できないようにする。

名前付き型はunderlying typeを展開せず、宣言の完全修飾名で区別する。\
generic instanceは宣言名とargument列を再帰的に記録し、名前付きの再帰型は解決途中の`Pending`でも宣言名で停止する。\
interfaceは名前付き宣言として扱う。

## methodとoperatorの参照

generic methodのsymbolはowner・receiver宣言名・method名・型argument列から生成する。\
generic operatorはowner・operator種別・receiverを含むparameter型列・型argument列で区別し、通常のoperatorも同じ型encodingをparameter列へ適用する。\
宣言配列のindexはlink symbolに含めない。\
symbolの先頭にあるownerのpackage path全体にもbyte長を付けるため、path中にsymbol markerやencodingに似た文字列が含まれていても別ownerと衝突しない。\
さらに、この3種類のsymbolは識別子や数字に含まれない`;`で終端し、通常関数・通常method・generic free functionのsymbolと区別する。

具体化を要求する際には、stableなsymbolをkeyとして現在の宣言indexと`TypeId`列をcompiler内のregistryへ記録する。\
本体生成はこの記録を利用するため、symbolを解析して一時的なIDへ復元する必要がない。\
registryはHIR構築中だけに存在し、公開HIR・成果物・metadataの形式へ追加しない。

method呼出、interfaceのvtable entry、公開method情報は同じsymbolを参照する。\
vtable自体の名前も安定した型名を利用するため、既存の具体化集合を持つownerのcacheを再利用し、callerだけを再構築しても参照が一致する。

## 回帰検査と対象範囲

compiler-coreの検査は、無関係なpackageによる実際のID変化、構造的なargumentの一致と相違、非公開fieldのowner、再帰型、generic interface、宣言順の変更を扱う。\
CLIの検査では公開metadataとmethod symbolの一致に加え、method・vtable・generic operator・通常operatorを使う混在cacheのlink結果を同じsourceのfresh buildと比較する。

generic free functionも宣言owner・関数名・型argument列からstable symbolを生成し、HIR構築中のprivate registryで現在の宣言と型IDへ対応付ける。\
CLIはowner packageが生成するfunction/static symbolと、そのMIRから参照される型構造、生成定数の値、公開interfaceをbuild identityへ含める。\
`sizeof(T)`や`alignof(T)`の評価後に型argumentがMIRから消えても、評価結果の定数値を追跡して古いObjectの再利用を防ぐ。\
呼出元で具体化要求が増えた場合はownerのObjectを再生成し、既存要求だけならownerを再利用する。\
project全ソースのfingerprintは、全packageが同じsource状態か確認してfrontendを省略するprivate cache manifestにだけ使う。
