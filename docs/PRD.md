# PRD Solease MVP

## 0. 文档信息

* 版本：v0.2（MVP-Concrete）
* 范围：黑客松 MVP（Pool 快贷 + P2P Offer + 到期拍卖清算 + Identity Showcase）
* 关键约束：

  * **仅支持 unwrapped / 非 tokenized 的 `.sol` 域名**作为抵押品
  * 抵押流程拆分为 **两笔交易**：`setup_collateral` → `verify_and_withdraw`
  * 清算结算时必须 **清理/重写 SOL record（至少 V1 delete）**
  * 不做复杂预言机，不做动态追加保证金清算（MVP 用“到期 + 拍卖”）

---

## 1. 背景与目标

### 1.1 背景

`.sol` 域名具备身份价值（收款地址、名片、社交绑定），但流动性差。用户常见需求是“短期周转”而不是卖掉域名。另一方面，LP/出借人希望获得收益，但需要明确的违约处理路径。

### 1.2 产品目标（MVP）

1. **Borrower** 能用 `.sol` 抵押借到资金（Pool 小额快贷 / P2P 大额灵活）
2. **LP** 能存入资金获得利息（来自 Borrower 偿还）
3. **违约可被确定性处理**：到期 → 宽限期 → 拍卖 → 结算转移域名与资金
4. **可用性主张不自打脸**：

   * 抵押期间钱包顶部名字可能消失（reverse），但前端要展示“抵押中的身份”
   * 避免 record 残留导致新买家“拿了域名却钱还打到旧地址”的坑

### 1.3 成功标准（可验收）

* 端到端流程可在本地/Devnet 完整跑通：

  * Pool：LP deposit → Borrower setup → withdraw → repay → LP withdraw
  * P2P：Lender offer → Borrower accept → repay 或 default → auction → settle
* 关键拒绝条件生效：

  * tokenized/wrapped 域名抵押被拒绝
  * 发现 registrar 风险（见后）被拒绝
  * 同一域名重复借款被拒绝
* 拍卖结算后：

  * 域名 owner 转移给赢家
  * 债权方拿到应得资金
  * SOL record 被清理/重写（至少 V1 删除）

---

## 2. 用户与场景

### 2.1 用户角色

* Borrower：域名持有者
* LP：池子流动性提供者
* P2P Lender：点对点出借者
* Bidder/Liquidator：拍卖参与者

### 2.2 典型场景

* 场景 A（快）：Borrower 抵押 `alice.sol`，从 Pool 借小额（低风控上限），7-30 天到期归还
* 场景 B（灵活）：Borrower 想借更多，去 P2P 接受某 Lender 条款（利率/期限自定义）
* 场景 C（违约）：到期未还，进入 24h 宽限期（罚息飙升），仍未还则拍卖出清域名

---

## 3. 功能需求（MVP 详细）

### 3.1 抵押品准入与校验

#### 3.1.1 抵押品准入规则（链上硬校验）

1. 域名必须属于 Borrower（当前 registry.owner == borrower）
2. 域名必须为 **unwrapped / 非 tokenized**
3. 域名不存在活跃贷款（LoanAccount 状态非 Active/Grace/Auction）
4. 域名不存在 **Subdomain Registrar 风险**（MVP：检测 registrar account 是否存在，存在则拒绝；或 admin 不符合期望则拒绝）

> 验收：任意一条不满足，`setup_collateral` 直接失败并返回明确错误码。

---

### 3.2 Pool 快贷（Instant Pool Loan）

#### 3.2.1 LP 存取

* **Deposit**

  * 输入：amount（USDC）
  * 输出：mint shares（LP 份额）
  * 规则：shares = amount * (total_shares / total_assets)（MVP 可简化为初始 1:1，之后按资产净值）
* **Withdraw**

  * 输入：shares
  * 输出：USDC amount
  * 规则：若 vault 可用现金不足则失败（MVP 不做排队），或只允许提到可用上限

#### 3.2.2 Borrower 从 Pool 借款

* 期限：固定档（7/14/30 天）
* 借款上限（关键风控）：

  * `borrow_cap = min(global_cap, per_domain_cap_by_tier)`
  * **MVP 默认 cap 极保守**，避免 wash-trading 估值攻击
* 利率：

  * MVP 可用固定 APR（例如 30% APR）或极简 utilization 曲线
* 借款流程：见 3.4 “两步交易”

#### 3.2.3 Pool 违约处理

* 到期未还 → 宽限期（24h）
* 宽限期仍未还 → `start_auction` → 出清
* 拍卖所得优先偿还 Pool（本息 + 罚金），剩余返还 Borrower（可选，MVP 可为 0 以简化）

---

### 3.3 P2P Offer（挂单出借）

#### 3.3.1 Offer 创建/取消

* Lender 创建 Offer：

  * principal（USDC）、APR、duration、offer_expiry、可选 domain filter（只接受某个域名/某类域名）
  * **资金锁定**：创建 Offer 时把 principal 转入 OfferEscrow（PDA）
* 取消 Offer：未被接受时可取消并取回资金

#### 3.3.2 Borrower 接受 Offer

* Borrower 选择域名 + 选择 offer_id
* 触发：同样走 `setup_collateral` → `verify_and_withdraw`
* 放款来源：OfferEscrow（不是 Pool）

#### 3.3.3 P2P 违约处理

* 到期后同样进入宽限期 → 拍卖
* 拍卖所得优先偿还该 Lender（本息 + 罚金）

---

### 3.4 借款两步交易（强制）

#### 3.4.1 Step 1：setup_collateral（Tx1）

目的：**把域名控制权交给协议**，并完成借款前置检查。

* 输入：

  * domain_registry_pubkey
  * mode：Pool / P2P（若 P2P 则附 offer_id）
  * borrower_payout_pubkey（Borrower 想绑定的收款地址，用于 record 逻辑）

* 链上动作：

  1. 校验抵押品准入（3.1）
  2. 创建 LoanAccount（状态 = `SetupPending`，写入 due_ts、principal、apr、mode 等）
  3. 将域名 owner 转移到 Escrow PDA
  4. 标记 `record_policy`：

     * `REQUIRE_CLEAR_ON_SETTLE = true`
     * `payout_pubkey = borrower_payout_pubkey`
  5. （可选）写入一个 `NeedRecordAck` 标志，要求前端提示用户设置/确认 SOL record

* 输出：loan_id（LoanAccount PDA）

#### 3.4.2 Step 2：verify_and_withdraw（Tx2）

目的：**确认抵押状态正确**，然后发放贷款。

* 输入：

  * loan_id
  * （若 P2P）offer_id
* 链上动作：

  1. 校验 LoanAccount 状态 = SetupPending
  2. 校验域名 owner 仍为 Escrow PDA（防中途反转）
  3. 校验资金来源：

     * Pool：PoolVault 有足够 USDC + 满足 buffer 规则
     * P2P：OfferEscrow 有足够 USDC 且 offer 未过期未被占用
  4. 放款：

     * 从对应 vault/escrow 转 USDC 给 Borrower（或 Borrower 的收款地址）
  5. 更新 LoanAccount：状态 = Active，start_ts = now

> 验收：Tx2 成功后，Borrower 收到 USDC，域名仍被 escrow，LoanAccount 状态 Active。

---

### 3.5 还款与解押

#### 3.5.1 repay（到期前或宽限期内）

* 输入：loan_id
* 链上动作：

  1. 校验状态 Active 或 Grace
  2. 计算应还金额：

     * principal + interest +（若 Grace）penalty_interest
  3. 从 Borrower 转入 USDC 到目标（PoolVault 或 Lender）
  4. 将域名 owner 从 Escrow PDA 转回 Borrower
  5. LoanAccount 状态 = Repaid

> MVP 简化：repay 后不处理 reverse lookup（前端展示即可）。

---

### 3.6 宽限期、罚金与清算拍卖

#### 3.6.1 enter_grace

* 触发条件：now >= due_ts 且状态 Active
* 状态转移：Active → Grace（记录 grace_start_ts）

#### 3.6.2 start_auction

* 触发条件：now >= due_ts + grace_period 且状态 Grace
* 创建 AuctionAccount（状态 AuctionLive）
* 设置参数：

  * `auction_end_ts`
  * `min_bid = debt_owed`（或 debt_owed 的某个折价，MVP 建议 = debt_owed）
  * `buy_now_start_price`、`buy_now_end_price`、`buy_now_end_ts`（线性下降）

#### 3.6.3 place_bid

* 校验 now < auction_end_ts
* bid_amount >= max(min_bid, highest_bid + min_increment)
* 锁定 bidder 资金到 BidEscrow（若已有最高出价，先退回旧最高出价）
* 更新 highest_bid / highest_bidder

#### 3.6.4 buy_it_now（兜底出清）

* 当前价格：按线性函数从 start_price 下降到 end_price（可为 0 或极低）
* 若有人支付当前价：

  * 直接成为 winner，进入可 settle 状态
  * 退回最高出价人资金（若存在）

#### 3.6.5 settle_auction（必须做 record 清理）

* 触发条件：

  * now >= auction_end_ts 或 buy_it_now 已触发
* 动作：

  1. winner 付款已在 escrow
  2. 资金分配：

     * Pool 模式：先补回 PoolVault（本金+利息+罚金），剩余返 Borrower（MVP 可不返）
     * P2P 模式：先给 Lender（本金+利息+罚金）
  3. 域名转移：Escrow PDA → winner
  4. **SOL record 清理/重写**：

     * MVP 最低要求：删除 SOL record（V1 delete）
     * 若不支持 delete，则强制重写为 winner（需要 winner 签名，可能拆成结算后第二步）
  5. LoanAccount 状态 = Settled（Defaulted）

---

### 3.7 前端需求（MVP 必要页面）

1. 首页：模式入口（Pool / P2P）、TVL、未偿还贷款数
2. Borrow 页面：

   * 选择域名
   * Pool 借款额度展示（cap）
   * P2P offers 列表与筛选
   * 两步交易引导（Tx1/Tx2）
3. Lend Pool 页面：

   * deposit/withdraw
   * 利率与利用率（MVP 可只显示固定 APR）
4. P2P Lend 页面：

   * 创建 offer、取消、查看已成交
5. Auction 页面：

   * 当前最高价、Buy-it-now 当前价、结束时间
6. **My Staked Identity（Showcase Mode）**

   * 展示：你抵押中的域名、到期时间、状态、可操作按钮（repay/进入拍卖）

---

## 4. 非功能需求（MVP）

* 安全：

  * 合约不允许管理员任意挪用抵押品
  * 每个状态迁移必须有清晰前置条件
* 可观察性：

  * 每个关键动作发 event（LoanCreated/Activated/Repaid/Default/AuctionStarted/BidPlaced/Settled）
* 兼容性：

  * 明确提示：抵押期间钱包显示名可能消失，但收款解析依赖 record/应用实现

---

