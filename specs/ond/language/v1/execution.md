# Memory、初期化、entry
<!-- ond-spec-id: language/v1-execution -->

本章は言語levelのmemory契約、初期化、通常build entryを定義する。\
target固有の挙動はtarget profileを参照する。

## メモリ管理
<!-- ond-spec-id: runtime/memory-management -->

- address を取得した local variable の storage はその関数 invocation の stack frame、package-level variable の storage は startup segment に置く
  - compiler は escaping local variable を暗黙に heap へ移さない
- heap allocation は組み込みの `new(T)` または `alloc[T](count)` でだけ明示的に行う
- `new(T)` は `T` 1 個分を `alignof(T)` に揃えて確保し、zero value で初期化した `*T` を返す
- `alloc[T](count)` は `T` を `count: u32` 個格納できる連続領域を `alignof(T)` に揃えて確保し、`*T` を返す
  - 初期byte値の選択は実装裁量とする
- `new` と `alloc` は allocation failure 時に `nil` を返す
  - `alloc[T](0)` は `nil` を返す
  - zero-size 型を正の個数確保した成功結果は non-nil とする
- `free(pointer)` は `new` または `alloc` が返した未解放の先頭 pointer、または `nil` を受け取る
  - `free(nil)`はallocation状態を保持して正常に完了する
- heap allocation の所有権は pointer を保持するプログラム側にあり、不要になった領域を `free` する
- raw pointer accessの有効領域、生存期間、freeへ渡す未解放先頭pointerは利用側の成立前提である

bounds checkの失敗は追加のstack frameやheap allocationに依存しない。\
具体的なfault命令とdebug metadataはtarget仕様に従う。

## 初期化とエントリ
<!-- ond-spec-id: runtime/initialization-and-entry -->

- package-level `var` は initializer を省略した zero value、または compile-time constant initializer だけを持てる
  - runtime 呼び出しを伴う initializer は compile error とする
- compilerはglobal initializerを適用する内部関数を生成できる
  - sourceの`init`は次項の通常関数規則に従う
- `init`は通常の関数名として宣言し、明示callで起動する
- 通常buildの`main` packageはparameterと戻り値を持たない`func main()`をちょうど1個宣言する
- `main` の return は正常終了 code `0` とする

package内のsource fileをファイル名の辞書順で巡回し、各fileのimportをsource記述順に深さ優先で訪問する。\
各packageは一度だけ初期化し、依存先の初期化を先に完了する。\
package内のglobal initializerはsource file順、各fileの宣言順に適用する。\
通常buildではrootから到達するpackageのinitializerを実行する。\
全globalのzero初期化を完了した後、この順序でinitializerを適用し、最後に`main.main`を呼び出す。

ここで定めるentryは通常buildのentryである。\
test buildの個別起動入口は[テストビルド仕様](../../build/test-build.md)を参照する。

Kagura target の runtime entry と正常終了通知は [Ond for Kagura Machine Profile v1](../../targets/kagura-v1/machine.md) に従う。

## Build Output
<!-- ond-spec-id: build/output -->

- compilerはsourceから[Object](../../artifacts.md)を生成し、linkerは[linking契約](../../linking.md)に従いLinkedImageを生成する
  - 配布ツールはLinkedImageを配布形式またはROMへまとめる
- assembly は中間生成物または診断用出力として生成できる
- 全packageとruntimeは静的にlinkし、呼出側が指定する配置値に従ってLinkedImageを生成する
- build toolはasset discoveryとresource IDを管理し、実行環境はdevice protocolとmemory mappingを定める
- 配布ツールはLinkedImageとassetを配布形式へ同梱する
- 正常終了後の制御経路はcallerが指定するtermination instructionsに従う
