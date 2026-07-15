import type { DeployProgress } from '../types';

export function isInstanceDeploying(
  instanceId: string,
  deployProgress?: DeployProgress | null
): boolean {
  return (
    !!deployProgress &&
    deployProgress.instance_id === instanceId &&
    deployProgress.step !== 'done' &&
    deployProgress.step !== 'error'
  );
}
