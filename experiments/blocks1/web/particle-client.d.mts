export class ParticleClient {
  constructor(options: { baseUrl: string });
  readonly busy: boolean;
  generate(
    workflow: unknown,
    text: string,
    options?: {
      steps?: number;
      seed?: number;
      timeoutMs?: number;
      signal?: AbortSignal;
      onEvent?: (event: { type: string }) => void;
    },
  ): Promise<{ blob: Blob; promptId: string }>;
}
