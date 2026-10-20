export type AuthState = "ok" | "login_required" | "error";
export type AuthMode = "token" | "password";
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
  creditExpiries: CreditExpiry[];
}

export interface CreditExpiry {
  amount: number;
  at: number;
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

export interface KeyUsageView {
  id: number;
  name: string;
  prefix: string;
  enabled: boolean;
  todayTokens: number;
  totalTokens: number;
}

export interface AppSnapshot {
  updatedAt: number;
  lastSuccessAt: number;
  stale: boolean;
  authState: AuthState;
  username: string;
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
  isAdmin: boolean;
  keyUsage: KeyUsageView[];
  error: string | null;
}

export interface HomepageSettings {
  singleColumn: boolean;
  showCreditOverview: boolean;
  showUsage: boolean;
  showAccounts: boolean;
  showKeys: boolean;
}

export interface SettingsView extends HomepageSettings {
  baseUrl: string;
  authMode: AuthMode;
  token: string;
  username: string;
  password: string;
  refreshIntervalSec: number;
  lowCreditThreshold: number;
  launchAtLogin: boolean;
  configPath: string;
}

// 保存设置时登录方式由登录页决定，这里只提交当前方式对应的凭据与监测项。
export type SettingsInput = Omit<SettingsView, "configPath" | "authMode">;
