import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import {
  PublicKey,
  Keypair,
  SystemProgram,
  LAMPORTS_PER_SOL,
  SYSVAR_RENT_PUBKEY,
} from "@solana/web3.js";
import {
  TOKEN_PROGRAM_ID,
  ASSOCIATED_TOKEN_PROGRAM_ID,
  createMint,
  createAccount,
  mintTo,
  getAssociatedTokenAddress,
  createAssociatedTokenAccount,
} from "@solana/spl-token";

import { CPAMMHelper } from "./cp_amm_helper.js";
import { StreamflowHelper } from "./streamflow_helper.js";

// Real CP-AMM and Streamflow program IDs
export const CP_AMM_PROGRAM_ID = new PublicKey("675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8");
export const STREAMFLOW_PROGRAM_ID = new PublicKey("strmRqUCoQUgGUan5YhzUZa6KqdzwX5L6FpUxfmKg5m");

// Test constants
export const DAY_SECONDS = 86400;
export const MAX_BASIS_POINTS = 10000;

export class TestEnvironment {
  provider: anchor.AnchorProvider;
  program: Program<any>;
  cpAmmHelper!: CPAMMHelper;
  streamflowHelper!: StreamflowHelper;

  // Test accounts
  authority!: Keypair;
  user1!: Keypair;
  user2!: Keypair;
  user3!: Keypair;

  // Token mints
  baseMint!: PublicKey;
  quoteMint!: PublicKey;

  // CP-AMM pool related
  pool!: Keypair;
  poolAuthority!: PublicKey;
  baseVault!: PublicKey;
  quoteVault!: PublicKey;

  // Honorary position
  position!: Keypair;
  positionNftMint!: PublicKey;
  positionNftAccount!: PublicKey;

  // Policy and progress accounts
  policy!: PublicKey;
  progress!: PublicKey;
  honoraryPosition!: PublicKey;

  // Treasury accounts
  quoteTreasury!: PublicKey;
  baseFeeCheck!: PublicKey;

  // Creator ATA
  creatorQuoteAta!: PublicKey;

  // Investor ATAs
  investor1QuoteAta!: PublicKey;
  investor2QuoteAta!: PublicKey;
  investor3QuoteAta!: PublicKey;

  constructor() {
    this.provider = anchor.AnchorProvider.env();
    anchor.setProvider(this.provider);
    this.program = anchor.workspace.HonoraryQuoteFee as Program<any>;
  }

  async initialize() {
    // Initialize helpers
    this.cpAmmHelper = new CPAMMHelper(this.provider);
    this.streamflowHelper = new StreamflowHelper(this.provider);

    // Generate test keypairs
    this.authority = Keypair.generate();
    this.user1 = Keypair.generate();
    this.user2 = Keypair.generate();
    this.user3 = Keypair.generate();

    this.pool = Keypair.generate();
    this.position = Keypair.generate();

    // Airdrop SOL to test accounts
    await this.airdropTo(this.authority.publicKey, 10 * LAMPORTS_PER_SOL);
    await this.airdropTo(this.user1.publicKey, 10 * LAMPORTS_PER_SOL);
    await this.airdropTo(this.user2.publicKey, 10 * LAMPORTS_PER_SOL);
    await this.airdropTo(this.user3.publicKey, 10 * LAMPORTS_PER_SOL);

    // Create test token mints
    this.baseMint = await createMint(
      this.provider.connection,
      this.authority,
      this.authority.publicKey,
      null,
      9
    );

    this.quoteMint = await createMint(
      this.provider.connection,
      this.authority,
      this.authority.publicKey,
      null,
      9
    );

    // Create creator quote ATA
    this.creatorQuoteAta = await createAssociatedTokenAccount(
      this.provider.connection,
      this.authority,
      this.quoteMint,
      this.authority.publicKey
    );

    // Mint initial tokens
    await mintTo(
      this.provider.connection,
      this.authority,
      this.quoteMint,
      this.creatorQuoteAta,
      this.authority,
      1000000000 // 1000 quote tokens
    );

    await mintTo(
      this.provider.connection,
      this.authority,
      this.baseMint,
      await getAssociatedTokenAddress(this.baseMint, this.authority.publicKey),
      this.authority,
      1000000000 // 1000 base tokens
    );

    // Create investor quote ATAs
    this.investor1QuoteAta = await createAssociatedTokenAccount(
      this.provider.connection,
      this.authority,
      this.quoteMint,
      this.user1.publicKey
    );

    this.investor2QuoteAta = await createAssociatedTokenAccount(
      this.provider.connection,
      this.authority,
      this.quoteMint,
      this.user2.publicKey
    );

    this.investor3QuoteAta = await createAssociatedTokenAccount(
      this.provider.connection,
      this.authority,
      this.quoteMint,
      this.user3.publicKey
    );

    // Derive PDAs
    [this.policy] = PublicKey.findProgramAddressSync(
      [Buffer.from("policy"), this.pool.publicKey.toBuffer()],
      this.program.programId
    );

    [this.progress] = PublicKey.findProgramAddressSync(
      [Buffer.from("progress"), this.pool.publicKey.toBuffer()],
      this.program.programId
    );

    [this.honoraryPosition] = PublicKey.findProgramAddressSync(
      [Buffer.from("honorary"), this.policy.toBuffer()],
      this.program.programId
    );

    [this.poolAuthority] = PublicKey.findProgramAddressSync(
      [Buffer.from("authority")],
      CP_AMM_PROGRAM_ID
    );

    // Create vault ATAs (simplified for testing)
    this.baseVault = await createAccount(
      this.provider.connection,
      this.authority,
      this.baseMint,
      this.poolAuthority
    );

    this.quoteVault = await createAccount(
      this.provider.connection,
      this.authority,
      this.quoteMint,
      this.poolAuthority
    );

    // Create position NFT mint
    this.positionNftMint = await createMint(
      this.provider.connection,
      this.authority,
      this.honoraryPosition,
      null,
      0
    );

    this.positionNftAccount = await createAccount(
      this.provider.connection,
      this.authority,
      this.positionNftMint,
      this.honoraryPosition
    );

    // Create treasury accounts
    this.quoteTreasury = await getAssociatedTokenAddress(this.quoteMint, this.honoraryPosition);
    this.baseFeeCheck = await getAssociatedTokenAddress(this.baseMint, this.honoraryPosition);
  }

  async airdropTo(publicKey: PublicKey, amount: number) {
    const tx = await this.provider.connection.requestAirdrop(publicKey, amount);
    await this.provider.connection.confirmTransaction(tx);
  }

  async createMockPoolAccount() {
    // Create a mock pool account with minimal data structure
    // In a real test, this would be created by the CP-AMM program
    const poolData = {
      tokenAMint: this.baseMint,
      tokenBMint: this.quoteMint,
      tokenAVault: this.baseVault,
      tokenBVault: this.quoteVault,
      authority: this.poolAuthority,
      partner: PublicKey.default, // No partner for testing
    };

    // For testing purposes, we'll use the pool keypair account
    // In reality, this would be created by CP-AMM program
    return poolData;
  }

  async createMockPositionAccount() {
    // Create a mock position account
    const positionData = {
      pool: this.pool.publicKey,
      feeAPending: 0,
      feeBPending: 0,
      unlockedLiquidity: 0,
      vestedLiquidity: 0,
      permanentLockedLiquidity: 0,
    };

    return positionData;
  }

  /**
   * Create mock Streamflow vesting contract
   * This simulates a Streamflow contract structure for testing
   */
  async createMockStreamflowContract(
    recipient: PublicKey,
    recipientTokens: PublicKey,
    totalAmount: number,
    withdrawnAmount: number,
    startTime: number,
    endTime: number
  ): Promise<Keypair> {
    const contract = Keypair.generate();
    
    // In a real implementation, this would create actual Streamflow contract data
    // For testing, we just need a keypair that represents the contract
    // The actual data would be serialized Streamflow contract state
    
    return contract;
  }

  /**
   * Helper to calculate mock locked amount
   * Simulates Streamflow's locked_amount calculation
   */
  calculateMockLockedAmount(
    totalAmount: number,
    withdrawnAmount: number,
    availableToClaim: number
  ): number {
    const unlocked = withdrawnAmount + availableToClaim;
    const clamped = Math.min(unlocked, totalAmount);
    return Math.max(0, totalAmount - clamped);
  }

  /**
   * Initialize real CP-AMM pool with quote-only fees
   * This uses the actual CP-AMM program
   */
  async initializeRealPool(): Promise<void> {
    console.log("\n🏊 Initializing REAL CP-AMM Pool...");
    
    const poolResult = await this.cpAmmHelper.initializePool({
      payer: this.authority,
      baseMint: this.baseMint,
      quoteMint: this.quoteMint,
      feeRate: 30, // 0.3% fee
      quoteOnlyFees: true,
    });

    this.pool = poolResult.pool;
    this.poolAuthority = poolResult.poolAuthority;
    this.baseVault = poolResult.baseVault;
    this.quoteVault = poolResult.quoteVault;

    console.log("✅ Real CP-AMM pool initialized!");
  }

  /**
   * Create real position with fees
   * This simulates actual trading fees accumulation
   */
  async createRealPositionWithFees(feeAmount: number): Promise<void> {
    console.log("\n📍 Creating Real Position with Fees...");

    // Create the position
    const positionResult = await this.cpAmmHelper.createPosition({
      payer: this.authority,
      pool: this.pool.publicKey,
      poolAuthority: this.poolAuthority,
      owner: this.honoraryPosition,
      lowerTick: -100,
      upperTick: 100,
    });

    this.position = positionResult.position;
    this.positionNftMint = positionResult.positionNftMint;
    this.positionNftAccount = positionResult.positionNftAccount;

    // Inject fees directly (simpler than running actual swaps)
    await this.cpAmmHelper.injectFeesDirectly({
      payer: this.authority,
      position: this.position.publicKey,
      quoteVault: this.quoteVault,
      quoteMint: this.quoteMint,
      feeAmount: new anchor.BN(feeAmount),
    });

    console.log(`✅ Position created with ${feeAmount} quote fees!`);
  }

  /**
   * Create real Streamflow vesting contracts
   * Returns array of contract data for use in crank
   */
  async createRealVestingContracts(scenarios: Array<{
    recipient: PublicKey;
    totalAmount: number;
    lockedPercent: number; // 0-100
  }>): Promise<Array<{
    contract: Keypair;
    recipientAta: PublicKey;
    lockedAmount: number;
  }>> {
    console.log("\n🌊 Creating Real Streamflow Vesting Contracts...");

    const results = [];
    const now = new anchor.BN(Math.floor(Date.now() / 1000));
    const oneDay = new anchor.BN(86400);

    for (const scenario of scenarios) {
      console.log(`\n  Contract for ${scenario.recipient.toString().slice(0, 8)}...`);
      console.log(`    Total: ${scenario.totalAmount}`);
      console.log(`    Locked: ${scenario.lockedPercent}%`);

      // Calculate time parameters to achieve desired locked percentage
      const vestingDuration = 60; // 60 days total
      const daysElapsed = Math.floor(vestingDuration * (1 - scenario.lockedPercent / 100));
      
      const startTime = now.sub(oneDay.muln(daysElapsed));
      const endTime = now.add(oneDay.muln(vestingDuration - daysElapsed));

      const recipientAta = await getAssociatedTokenAddress(
        this.quoteMint,
        scenario.recipient
      );

      // Check if ATA exists, create if not
      try {
        await this.provider.connection.getAccountInfo(recipientAta);
      } catch {
        await createAssociatedTokenAccount(
          this.provider.connection,
          this.authority,
          this.quoteMint,
          scenario.recipient
        );
      }

      const contract = await this.streamflowHelper.createMockContractData({
        payer: this.authority,
        recipient: scenario.recipient,
        recipientTokens: recipientAta,
        mint: this.quoteMint,
        totalAmount: new anchor.BN(scenario.totalAmount),
        withdrawnAmount: new anchor.BN(0),
        startTime,
        endTime,
      });

      const lockedAmount = Math.floor(scenario.totalAmount * (scenario.lockedPercent / 100));

      results.push({
        contract,
        recipientAta,
        lockedAmount,
      });

      console.log(`    ✓ Contract created: ${contract.publicKey.toString().slice(0, 8)}...`);
      console.log(`    ✓ Locked amount: ${lockedAmount}`);
    }

    console.log("\n✅ All vesting contracts created!");
    return results;
  }
}
