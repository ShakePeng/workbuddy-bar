export type AuthState = "ok" | "login_required" | "error";
export type AccountStatus = "healthy" | "cooling" | "disabled" | "expired" | "unavailable" | "unknown";

export interface AccountView {
  file: string;
  uid: string;
  nickname: string;
  realm: string;
  credits: number | null;
  status: AccountStatus;
  healthy: boolean | null;
  cooling: boolean;
  disabled: boolean;
  manualDisabled: boolean;
  expiresAt: number | null;
  isExpired: boolean;
  inPool: boolean | null;
  poolUnknown: boolean;
  source: string;
}

export interface UsageSummary {
  todayRequests: number;
  todayTokens: number;
  todayCredit: number;
  weekRequests: number;
  weekTokens: number;
  weekCredit: number;
  totalRequests: number;
  totalTokens: number;
  totalCredit: number;
  activeKeys: number;
  topModel: string | null;
}

export interface UsagePoint {
  day: string;
  requests: number;
  tokens: number;
  credit: number;
  failed: number;
}

export interface AppSnapshot {
  updatedAt: number;
  lastSuccessAt: number;
  stale: boolean;
  authState: AuthState;
  poolAvailable: boolean;
  poolSynced: number;
  accountTotal: number;
  enabledAccounts: number;
  creditSyncedAccounts: number;
  creditPartial: boolean;
  totalCredits: number | null;
  accounts: AccountView[];
  today: UsageSummary;
  usageDays: UsagePoint[];
  error: string | null;
}

export interface SettingsView {
  baseUrl: string;
  username: string;
  password: string;
  refreshIntervalSec: number;
  lowCreditThreshold: number;
  launchAtLogin: boolean;
  configPath: string;
}
