# 7.1 Start

你是我的 Solana/Anchor 工程合伙人。目标是在黑客松周期内实现一个 `.sol` 域名抵押借贷 MVP，支持两种模式：

- **Pool 快贷**：LP 存入 USDC 获收益；Borrower 抵押 *unwrapped* `.sol` 域名借出小额 USDC，强风控 cap 防估值攻击；
- **P2P Offer**：Lender 挂单，Borrower 接单，固定期限/利率。

违约后进入拍卖：**English bid + Buy-it-now 价格线性下降兜底出清**。

---

### 关键约束

- 只支持 **unwrapped、非 tokenized** 的 `.sol` 域名作为抵押品；
- 检测到 `registrar` 或 `admin` 权限残留 → 拒绝抵押（防控制权篡改）；
- 代码坚持 **TDD**：先写测试 → 再写实现；所有状态机迁移必须可测；
- `setup_collateral` 与 `verify_and_withdraw` 分成两笔交易（降低 compute 单元风险）；
- 结算时必须清理/重写 SOL record：至少实现调用 `deleteInstruction` 的链上接口或等价行为（如事件标记 + 后处理说明）；

---

### 请先输出：

1. 仓库结构  
2. Program 账户模型（Account structs）  
3. 指令列表（Instructions）  
4. 测试用例清单（逐条可执行）

→ 再开始编码。

每写完一个指令：
- 先补测试
- 再补实现
- 提交前确保 `anchor test` 通过

---

# 7.2 建议测试流程

运行 `anchor test` 覆盖以下路径：

| 类型       | 场景 |
|------------|------|
| 正常路径   | 抵押 → 借款 → 还款 → 解押（Pool & P2P） |
| 违约路径   | 到期 → 宽限期 → 拍卖启动 → 出价 → Buy-it-now 触发 → 结算 |
| 拒绝路径   | tokenized 域名 / 存在 registrar 权限 / 重复借款 / 超 cap / 非 owner 抵押 |
| 集成测试（可选 devnet） | 跑通一条 happy path，确保 demo 可展示 |

---

# 7.3 TDD 用例清单

## 一、抵押设置与验证

- [ ] `setup_collateral` 成功后：domain owner == escrow PDA，LoanAccount 状态为 `Collateralized`，记录域名、时间戳、验证标志
- [ ] `verify_and_withdraw` 失败：未 setup 时调用 → revert
- [ ] `verify_and_withdraw` 成功：setup 后调用 → vault 创建，USDC 正确划转，LoanAccount 变为 `Active`

## 二、Pool 快贷

- [ ] `deposit`：LP 存入 USDC，share 增加正确，total liquidity 更新
- [ ] `withdraw`：LP 取回，按 share 比例计算，余额变化正确
- [ ] `borrow_from_pool`：cap 机制生效（单域名最大额度限制），超过则 revert
- [ ] 池子余额不足时 borrow 失败

## 三、P2P Offer

- [ ] `create_offer`：lender 发布 offer，OfferAccount 创建，字段正确（amount, rate, duration）
- [ ] `cancel_offer`：lender 撤单，account 关闭，rent 返回
- [ ] `accept_offer`：borrower 接单，LoanAccount 初始化，资金从 lender vault 划出
- [ ] 到期利息计算正确（simple interest: `principal * rate * days / 365`）

## 四、清算与拍卖

- [ ] `start_auction`：仅到期未还时可调用，LoanAccount 状态变为 `Auction`
- [ ] `place_bid`：出价高于当前最高价才成功，更新 bid info
- [ ] `buy_it_now`：价格随时间线性下降（公式：`start_price - decay * hours_elapsed`），调用即成交
- [ ] 拍卖期间禁止还款

## 五、结算

- [ ] `settle_loan`（正常还款）：本金+利息归还 lender/pool，域名 owner 回原主，LoanAccount 关闭
- [ ] `settle_auction`（违约拍卖）：最高出价者获得域名，资金按优先级分配（lender → LP → protocol fee）
- [ ] 结算时触发 `deleteInstruction` 调用 或 emit event 表明已清理 SOL record（如: `RecordCleared(domain, pubkey)`）
- [ ] 域名 owner 正确转移给 winner / original owner

## 六、安全校验

- [ ] 抵押时检测：`.sol` 域名必须 **unwrapped**（不是 Metadata-minted NFT）
- [ ] 检测 registrar 或 admin key 存在 → revert
- [ ] 同一域名不能重复抵押（LoanAccount 已存在则拒绝）
- [ ] 非 owner 尝试抵押 → revert

