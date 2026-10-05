# F20 ホスト接続の実現可能性調査

状態: 実験用fixtureによる予備調査。host ABI、接続profile、実装方式の採用を承認する文書ではない。調査日: 2026年10月5日。

本調査は[device組立境界の設計案](device-assembly.md)と[machine roadmap](machine-roadmap.md)にあるF20の論点を既存Kagura API上で試す。Kagura bus契約は[固定revisionのbus.md](https://github.com/SUPER-SHRINE/kagura/blob/457a48502f6726a8ac617561579bdd43670d0387/specs/kagura/v1/bus.md)、命令fault契約は[cpu.mdのfault節](https://github.com/SUPER-SHRINE/kagura/blob/457a48502f6726a8ac617561579bdd43670d0387/specs/kagura/v1/cpu.md#9-fault処理)を参照した。一回のbus accessのcommit保証と、fault命令全体でのdevice副作用なしは別の条件である。

Ond compilerの既存dev-dependencyで実際に解決するKagura 0.1.0を対象にした。`Cargo.lock`はKagura git revision `457a48502f6726a8ac617561579bdd43670d0387`を固定する。参照した固定版の[`Bus`](https://github.com/SUPER-SHRINE/kagura/blob/457a48502f6726a8ac617561579bdd43670d0387/support/reference/rust/kagura/src/bus.rs)は幅別read/writeを持つが、fetchとdataを区別する引数はない。固定版の[`Cpu::step`](https://github.com/SUPER-SHRINE/kagura/blob/457a48502f6726a8ac617561579bdd43670d0387/support/reference/rust/kagura/src/cpu.rs)は命令fetchにも`bus.read32(current_pc)`を呼ぶ。

fixtureは`crates/compiler/src/mir_backend/host_probe_tests.rs`にある単一の暫定Routerで、RAM `[0x1000, 0x1100)`と二つのProbe instanceを持つ。`gpu0`のregisters/vramはそれぞれ`0x8000`/`0x8100`、`gpu1`は`0x9000`/`0x9100`に置き、各windowは6 bytes。registersは幅4のみ、vramは幅1/2/4を許す。router eventにはinstance、window、offset、幅、read/write、値を記録する。これは静的な試験用modelであり、device仕様や実装ABIではない。

試験結果:

- gpu0のregistersへ書いてvramから読むと同じ値になり、同型のgpu1は独立した値を持つ。全6回のrouting eventでinstance/windowが一致する。
- routerはKaguraの幅別Bus callを幅そのまま一回のProbe操作へ配送する。実CPUがRAM内命令からLDB/LDH/LDWとSTB/STH/STWを個別に実行し、各々一つの幅一致eventになることを確認した。
- 両instanceのregistersへのbyte access、unaligned halfword、両vramのunaligned halfwordとoffset 4のword read/writeを拒否し、拒否eventを残さず、事前に設定した両instanceの値が変わらない。範囲・幅・alignmentをcallback前に検査する。
- partial setup fixtureでは二instance成功後のteardown順が`[2,1]`、二つ目の生成失敗時のcleanupが`[1]`になる。これは所有tokenを模したProbeのdrop記録で、native factory/module寿命の検証ではない。
- `step_guarded`は各CPU stepでfreshなFetchGuardを作り、最初のread32をfetchとしてRAM範囲内だけ許可する。read-clear MMIO fetchをcallback前にBusFaultにし、CPUのfetch fault metadataとCPU状態不変、read-clear状態不変、eventなしを確認した。RAM fetch後のMMIO LDWは通常のdata accessとしてread-clearを一度実行する。成功step後も次のMMIO fetchが拒否される。
- guardなしでread-clear MMIOから不正命令をfetchすると、InvalidInstruction fault時点でread-clearが一度commitしdevice値は0になる一方、CPU状態は維持される。Kaguraの命令fault全体の副作用なし条件は、各bus accessのatomicityだけでは保証できない。
- unaligned PCはCPUがBus呼出し前に`UnalignedPc`を返し、CPU状態・Probe状態・eventを変えない。

この試験が示すのは、直列に`Cpu::step`を呼ぶhost wrapperがfreshなfacadeを渡し、命令fetchを副作用のないRAM領域に制限するmachine policyを選ぶなら、Kagura 0.1.0の外側でMMIO fetchを抑止できるという条件付きの実現可能性である。試験した実行領域はRAMのみで、ROMは試していない。このmachine policyの採用は未承認である。任意の副作用MMIOからの命令実行は今回の非transactional経路では不適合である。この要件は、命令成功まで副作用を確定しない仕組みや取消しをdevice契約と合わせて検証するまで、後続実装をblockedとする。host側の追加契約で成立するか、Kagura側の変更が必要かは本試験では確定していない。Kagura変更が必要と判明した場合は、その依存だけを別途報告する。RAM限定案やF20全体はblockedとしない。

native shared libraryのload/unload、独立binary二実装の差替え、factory ABI、trust gate、実buffer寿命、service wiring、module隔離は未検証である。native moduleを第一候補とする既存計画を維持し、この静的fixtureをloaderや自由なdevice製品組立の実証と扱わない。接続profileは別途承認後に検証する。Kagura repository、生産コード、依存定義は変更していない。

再現コマンドはネットワークなしで実行した。

```text
cargo fmt --all -- --check
cargo test -p ond-compiler --locked --offline host_probe_tests
6 passed; 0 failed
git diff --check
```
