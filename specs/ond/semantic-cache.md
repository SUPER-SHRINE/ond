# パッケージ意味解析キャッシュ
<!-- ond-spec-id: build/semantic-cache -->

意味解析結果を再利用するキャッシュは、packageごとに通常sourceとtest sourceを別の単位として扱う。\
本書のキャッシュは同一process内で保持する意味解析結果を対象とし、永続化形式を定義しない。\
Object生成の永続キャッシュは[パッケージ成果物・独立コンパイラ契約](./package-build.md)に従う。

## 参照環境
<!-- ond-spec-id: build/semantic-cache-visibility -->

通常sourceの意味解析は、自packageとimportした依存packageの通常sourceだけを参照する。\
test sourceの意味解析は、自packageの通常sourceとtest source、およびimportした依存packageの通常sourceを参照環境とする。\
依存packageのtest sourceは、この環境へ含めない。\
名前の公開範囲、同一packageのprivate declarationへのアクセス、test sourceの判定は[テストビルド仕様](./test-build.md)に従う。

test sourceから通常functionを呼んでも、そのfunctionの定義とbodyは通常sourceの参照環境を維持する。\
通常sourceに定義されたgeneric function、method、operatorの特殊化にも、定義元の参照環境を適用する。\
型、型alias、constant、globalの解決にも同じ規則を使う。

通常sourceの型付きbodyと名前解決の検証結果は、test sourceの検証結果から独立して再利用できるものとする。\
test sourceの変更だけを理由に、変更されていない通常sourceの型付きbody全体を再検証しない。

## 同一性と失効
<!-- ond-spec-id: build/semantic-cache-invalidation -->

再利用の判定にはsourceの内容とidentity、および参照する通常sourceの依存関係を用いる。\
更新時刻だけを内容の同一性とみなさない。\
依存関係には型、constant、generic定義、および同一packageのprivate declarationを含め、公開signatureが同じであることだけでは再利用を認めない。

test sourceだけが変わった場合、対象packageのtest解析結果を失効させる。\
通常sourceと通常依存関係が不変なら、そのpackageと他packageの通常解析結果を再利用できる。\
他packageのtest解析結果は、参照環境が変わらない限り失効させない。

通常sourceが変わった場合、そのpackageの通常解析結果とtest解析結果、および変更された通常sourceを参照する解析結果を失効させる。\
importの追加・削除、fileの追加・削除・renameでも、変更後の参照環境に基づいて再利用を判定する。

解析結果と診断に含まれるsource ID、package ID、type IDは、その結果のsnapshotに属する。\
fileの列挙順などが変わったsnapshotへ、旧IDをそのまま適用してはならない。\
診断を再利用する場合は、primary labelと関連labelの両方を現在のsourceへ対応付ける。\
失敗した解析結果も同じ失効規則に従い、修正後に旧診断を残さない。

## 保持期間
<!-- ond-spec-id: build/semantic-cache-lifetime -->

意味解析キャッシュの保持期間は、それを所有するprocessまたは解析sessionの範囲内とする。\
キャッシュを破棄した場合も、再解析によって同じ意味解析結果を得られなければならない。\
LSPでは連続する編集の検証間でpackageごとの通常解析結果とtest解析結果を再利用する。\
再利用の有無によって可視性、診断、生成物の意味を変えない。
