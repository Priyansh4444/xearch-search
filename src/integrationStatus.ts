export type IndexingStatus = {
  xmd: boolean;
  indexing: boolean;
  handoff: boolean;
  collectorMode: "outbound" | "receiver";
};

export function indexingUnavailableMessage(config: IndexingStatus): string | undefined {
  if (config.indexing) return undefined;
  if (!config.xmd) return "Indexing needs an x.md key. Configure it in Connections.";
  if (config.collectorMode === "outbound" && !config.handoff)
    return "The download worker is offline. Imports will be available when it reconnects.";
  if (!config.handoff) return "Indexing needs a raw-capture receiver. Configure it in Connections.";
  return "Indexing is temporarily unavailable.";
}
