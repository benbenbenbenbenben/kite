// Dictionary demo: this file intentionally uses the forbidden term "Account"

export interface DictionaryDemo {
  id: string;
}

// This function name contains the forbidden term "Account"
export function listAccounts(): string[] {
  // In a real codebase, the dictionary rule would catch "Account" in identifiers
  return [];
}

// This class also uses the non-preferred term "User" instead of "KnownUser"
export class UserAccount {
  id: string = "";
  email: string = "";
}
