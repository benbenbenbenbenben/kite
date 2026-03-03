// FeatureFlagContext - FeatureFlag

export interface FeatureFlag {
  flagKey: string;
  enabled: boolean;
  rolloutPercentage: number;
}

export async function evaluateFlag(
  flagKey: string,
  userId: string,
): Promise<boolean> {
  void flagKey;
  void userId;
  return false;
}

export async function setFlagState(
  flagKey: string,
  enabled: boolean,
): Promise<FeatureFlag> {
  return { flagKey, enabled, rolloutPercentage: 100 };
}

export async function setRolloutPercentage(
  flagKey: string,
  percentage: number,
): Promise<FeatureFlag> {
  return { flagKey, enabled: true, rolloutPercentage: percentage };
}

export function assertRolloutPercentageMustBeWithinBounds(flag: FeatureFlag): void {
  if (flag.rolloutPercentage < 0 || flag.rolloutPercentage > 100) {
    throw new Error(
      `Rollout percentage must be 0-100, got ${flag.rolloutPercentage}`,
    );
  }
}

export function assertDisabledFlagsAlwaysEvaluateFalse(flag: FeatureFlag): void {
  if (!flag.enabled && flag.rolloutPercentage > 0) {
    throw new Error(
      "Disabled flags must not have a positive rollout percentage",
    );
  }
}
