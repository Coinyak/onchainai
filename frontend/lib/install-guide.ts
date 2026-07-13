/** Public install guide — barrel re-export. */
export type {
  ConnectGuideBlock,
  CopyGate,
  GuideLink,
  InstallPlatform,
  InstallSurfaceTool,
  PublicInstallGuide,
} from "./install-guide-shared";

export {
  ADD_MCP_INTENT,
  ALL_SELECTABLE_PLATFORMS,
  CONNECT_CARD_PLATFORMS,
  SITE_ORIGIN,
  blocksStructuredConfig,
  copyAllowed,
  copyGateForRisk,
  copyLabelAria,
  displayGuideText,
  installWarningText,
  platformAsStr,
  platformLabel,
} from "./install-guide-shared";

export {
  TOOL_INSTALL_CLIENTS,
  addMcpActionLabel,
  addMcpHref,
  addMcpHrefFromCompare,
  buildPublicInstallGuide,
  primaryInstallCommand,
  toolGuideMeta,
  toolHasInstallPath,
  toolInstallClientLabel,
  type ToolInstallClient,
} from "./install-guide-build";
