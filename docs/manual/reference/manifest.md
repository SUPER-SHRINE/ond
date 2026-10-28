# `ond.toml`リファレンス

全fieldの設定例を先に示し、後で用途や条件を説明します。

## 設定全体の例

次は19設定fieldを使ったmanifest例です。

```toml
# 実行project名とfinal projectの設定
[project]
name = "app" # project名は依存import pathの先頭にも使う
final = true # 依存先roleを指定でき、app自体は依存先にできない

# appで選べる追加role
[roles]
names = ["Debug", "Development"] # appで選べる追加role

# source探索から除外するdirectory
[resolve]
ignore = [".git", ".ond", "target"] # source探索から除くdirectory

# 依存先projectはそれぞれnon-finalとする
# math_developmentはroles.namesにDevelopmentを宣言するlibrary
[dependencies.math_local]
path = "../math" # path依存directory

[dependencies.math_development]
path = "../math-development" # path依存directory
# 追加構成の指定例。製品用ではこのrole行を省略する
role = "Development" # 依存先libraryで選ぶ追加role

# Git URLとrevisionは例示値
[dependencies.math_remote]
git = "https://github.com/example/math.git" # Git repository URL
rev = "v1.2.0" # 解決するrevision

# compile結果とpackage cacheの保存先
[compile]
output = "target/build.ondbuild" # CompilationManifest保存先
cache-dir = "target/ond-cache" # .ondpkg保存先

# Kagura実行fileの出力と起動設定
[link]
output = "target/app.bin" # 実行file保存先
startup = "external" # 外部loaderが起動する形式
stack-size = 8192 # stack予約量（byte）

# external形式で使うROM範囲
[link.rom]
base = 0 # ROM先頭address
size = 65536 # ROM容量（byte）

# 実行環境が用意するRAM範囲
[link.ram]
base = 65536 # RAM先頭address
size = 65536 # RAM容量（byte）

# main正常復帰後の実行環境側の移行先
[link.exit]
mode = "jump" # main復帰後にaddressへ移る
# modeをloopに変える場合は次のaddress行を削除する
address = 131072 # 実行環境に合わせて変更
```

依存先のdirectory、Git URL、revisionは例示値です。\
各projectを用意し、実際のpathやrepositoryへ置き換えてください。

address、memory map、起動方法、終了先も例示値です。\
実行環境のloaderや復帰先に合わせて調整してください。

## Projectとsource探索

| field | 用途と案内 |
| --- | --- |
| `project.name` | project名、依存projectのimport path先頭になる名前; identifier規則とsource探索は[packageとsource探索](../../../specs/ond/build/packages.md#manifestとsource探索)を参照 |
| `project.final` | trueなら他projectの依存先として登録不可; 依存先roleを明示できる条件にも関係（省略時false）; [final設定](../../../specs/ond/build/roles.md#final設定)と[依存edgeのrole選択](../../../specs/ond/build/roles.md#依存edgeのrole選択)を参照 |
| `roles.names` | projectが選べる名前付きrole一覧; 選択方法は[role仕様](../../../specs/ond/build/roles.md#manifest設定)を参照 |
| `resolve.ignore` | manifest directory相対のsource探索除外directory（既定値`.git`、`.ond`、`target`）; [source探索境界](../../../specs/ond/build/packages.md#source探索境界)を参照 |

`project.name`は実行projectとlibrary projectの両方に必要です。

`project.final`を省略した場合はfalseとして扱われます。

`[roles]`は名前付き追加roleを宣言する場合に使います。

role未指定構成は常にあり、`names`は追加構成を宣言します。

## Dependencies

依存登録nameごとに`dependencies.<name>`へ依存値を置きます。

`[dependencies]`のkeyは依存の登録名です。

依存sourceのimport path先頭には登録名ではなく、依存先manifestの`project.name`を使います。

依存の詳細形式で使うkeyは次のとおりです。

| key | 用途 |
| --- | --- |
| `path` | manifest相対または絶対の依存project directory |
| `git` | Git repository URL |
| `rev` | Gitで解決する任意revision |
| `role` | 依存先projectで選ぶrole |

### Path依存

path依存は、依存先projectのdirectoryを指定します。

相対pathは現在のmanifestのdirectoryを基準に解決します。

```toml
[dependencies]
math_local = { path = "../math" }
```

短いstring形式も使えます。

stringがURL形式ならGit依存、それ以外ならpath依存として扱われます。

```toml
[dependencies]
math_local = "../math"
```

### Git依存

Git依存はrepository URLと、必要なら`rev`を指定します。

この設定例はfield形式を示すだけで、取得操作は行いません。

```toml
[dependencies]
math_remote = { git = "https://github.com/example/math.git", rev = "v1.2.0" }
```

`rev`は省略できます。

Git依存に関係する解決済みrevisionは`ond.lock`に保存されます。

`ond.lock`は`ond.toml`のfieldではなく、Git依存の解決に使う別fileです。

lockのschemaと更新条件は[Git依存lock仕様](../../../specs/ond/build/packages.md#git依存lock-v2)を参照してください。

### 依存先role

詳細形式のpathまたはGit依存には、依存先projectで使うroleを指定できます。

`role`指定には、依存元projectの`project.final = true`と、依存先projectの`roles.names`に対象roleがあることが必要です。

role未指定は製品構成として使い、名前付きroleは共通sourceへ追加sourceを加える構成に使います。\
製品構成では依存設定の`role`も省略し、依存先の共通sourceを使います。

次の例は、appがfinal projectであり、mathlibが`Development` roleを宣言している場合の設定です。

```toml
[project]
name = "app"
final = true

[dependencies]
math_lib = { path = "../math", role = "Development" }
```

```toml
[project]
name = "mathlib"

[roles]
names = ["Development"]
```

roleは依存projectごとに選択します。

呼び出し元のroleが依存先へ自動伝播することはありません。

詳しい成立条件は[依存edgeのrole選択](../../../specs/ond/build/roles.md#依存edgeのrole選択)と[project境界](../../../specs/ond/build/roles.md#project境界)を参照してください。
## Compile設定

| field | 用途と既定値 |
| --- | --- |
| `compile.output` | CompilationManifestの保存先（省略時`target/build.ondbuild`） |
| `compile.cache-dir` | package `.ondpkg`成果物の保存directory（省略時`target/ond-cache`） |

`[compile]`table自体は省略できます。

compile単体では`[link]`tableを必要としません。

## Link設定

link設定は実行fileを作る場合に使います。

次の例はproject rootのmanifestに置く設定部分です。

`[link]`、`[link.ram]`、`[link.exit]`に必要な設定を置きます。

| field | 用途と既定値・条件 |
| --- | --- |
| `link.output` | 実行fileの保存先（省略時`target/a.bin`）; file出力では同じbasenameの`.onddebug` sidecarも生成 |
| `link.startup` | 起動形式`external`または`self`（必須） |
| `link.stack-size` | stack予約量をbyte単位で指定（必須） |
| `link.rom.base` | ROM address範囲の先頭（`[link.rom]`を置く場合に必要） |
| `link.rom.size` | ROM address範囲の容量（`[link.rom]`を置く場合に必要） |
| `link.ram.base` | RAM address範囲の先頭（必須） |
| `link.ram.size` | RAM address範囲の容量（必須） |
| `link.exit.mode` | main正常復帰後の経路（`loop`または`jump`、必須） |
| `link.exit.address` | `jump`先の絶対address（modeが`jump`の場合に指定） |

相対pathは、その`ond.toml`があるdirectoryを基準に解決します。

commandの`-o`を使う場合は対応するoutput設定をその呼出しに限って上書きし、manifestは書き換えません。

## 起動設定の例

### External

`external`形式は標準実行fileを出力し、外部loaderがload契約を満たしてentryへ制御を渡します。

`[link.rom]`を省略する例では、loaderが実行file全体をRAMへcopyする前提です。

```toml
[link]
startup = "external"
stack-size = 8192

[link.ram]
base = 4096
size = 65536

[link.exit]
mode = "loop"
```

external形式で`[link.rom]`を指定すると、Text/RodataはROM、Data/BssはRAMに配置します。

外部loaderは実行fileを保存元から読み、各領域を対応するmemoryへ配置します。
### Self

`self`形式では出力file先頭の起動処理がloaderの役割を担います。

`[link.rom]`が必要で、ROM baseは0にします。

```toml
[link]
startup = "self"
stack-size = 8192

[link.rom]
base = 0 # ROM先頭address
size = 65536

[link.ram]
base = 65536 # RAM先頭address
size = 65536

[link.exit]
mode = "loop"
```

`self`でも実行環境はROMへ出力を配置し、CPUをresetする必要があります。

設定条件の詳細は[起動方式](../../../specs/ond/artifacts.md#外部起動とセルフ起動)と[配置容量](../../../specs/ond/artifacts.md#配置容量と出力範囲)を参照してください。

### Jump終了先

`jump`はmainが正常に戻った後の移行先を絶対addressで指定します。

移行先の実行可能性とその後の動作は実行環境が定めます。

```toml
[link.exit]
mode = "jump" # main復帰後にaddressへ移る
address = 65536
```

`mode = "loop"`では`address`を置きません。

`mode = "jump"`では`address`が必要です。

## 設定の組合せと仕様

ROM設定は省略可能ですが、`startup = "self"`ではROM tableを置きbaseを0にします。

`startup = "external"`ではROMを省略するRAM copy形式も選べます。

数値範囲、alignment、ROM/RAMの重複、容量条件、起動・終了のwire動作はこのページで再定義しません。

[compile/link設定](../../../specs/ond/build/packages.md#compileとlink設定)、[実行fileの配置](../../../specs/ond/artifacts.md#配置容量と出力範囲)、[load状態](../../../specs/ond/artifacts.md#ロード完了時の状態)を参照してください。

