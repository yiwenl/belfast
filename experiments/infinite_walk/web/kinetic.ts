export interface SkeletonTarget {
  setSkeleton(points: Float32Array, timestampMs: number): void;
  clearSkeleton(): void;
}

export interface KineticPose {
  keypoints3D: ReadonlyArray<{ x: number; y: number; z: number; score?: number }>;
}

export function createSkeletonAdapter(target: SkeletonTarget) {
  const packed = new Float32Array(33 * 4);
  return {
    update(poses: readonly KineticPose[], timestampMs: number) {
      const points = poses[0]?.keypoints3D;
      if (!points || points.length !== 33) {
        target.clearSkeleton();
        return;
      }
      for (let i = 0; i < points.length; i++) {
        const { x, y, z, score = 0 } = points[i];
        if (![x, y, z, score].every(Number.isFinite) || score < 0 || score > 1) {
          target.clearSkeleton();
          return;
        }
        // Kinetic already applies its mirror option. Keep X; convert to Y-up, +Z toward viewer.
        packed.set([x, -y, -z, score], i * 4);
      }
      target.setSkeleton(packed, timestampMs);
    },
    clear() {
      target.clearSkeleton();
    },
  };
}
