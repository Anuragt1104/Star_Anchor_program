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

// Mock CP-AMM program for testing
export const CP_AMM_PROGRAM_ID = new PublicKey("675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8");

// Mock Streamflow program for testing - using a valid public key format
export const STREAMFLOW_PROGRAM_ID = Keypair.generate().publicKey;

// Test constants
export const DAY_SECONDS = 86400;
export const MAX_BASIS_POINTS = 10000;

export class TestEnvironment {
  provider: anchor.AnchorProvider;
  program: Program<any>;

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
      [Buffer.from("honorary-position"), this.policy.toBuffer()],
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
}
