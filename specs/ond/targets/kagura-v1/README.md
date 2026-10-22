# Ond for Kagura v1
<!-- ond-spec-id: targets/kagura-v1/index -->

この target profile は [Ond Language v1](../../language/v1/README.md) の program を Kagura v1 上で実行するための、Ond側の契約を定義する。

- [Target Runtime Contract](./machine.md)
- [ABI](./abi.md)
- [Objectとlinking](../../linking.md)

この profile は Ond に属し、Kagura 規格の一部ではない。

## 参照するKagura規範
<!-- ond-spec-id: targets/kagura-v1/kagura-normative-reference -->

本profileが参照するCPU/bus規範はKagura規範仕様1.0.0、snapshot commit `457a48502f6726a8ac617561579bdd43670d0387`とする。

- [版識別](https://github.com/SUPER-SHRINE/kagura/blob/457a48502f6726a8ac617561579bdd43670d0387/specs/README.md)
- [CPU ISA](https://github.com/SUPER-SHRINE/kagura/blob/457a48502f6726a8ac617561579bdd43670d0387/specs/kagura/v1/cpu.md)：instruction encoding、architectural state transition、fault処理
- [Bus契約](https://github.com/SUPER-SHRINE/kagura/blob/457a48502f6726a8ac617561579bdd43670d0387/specs/kagura/v1/bus.md)：accessとfault時のbus効果

Ondの型・数値意味、ABI、memory mapの指定、startup、成果物と正常終了経路は本profileとOnd各章の契約に従う。
