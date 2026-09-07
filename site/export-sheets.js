import { configProblems, generateRon } from "./ron-config.js";
import { announce, element, elements } from "./dom-elements.js";

const installerUrl = "https://raw.githubusercontent.com/Darkwing4/statusline-rs-cc/main/install.sh";

let lastDownloadUrl = "";

export function invalidateSession() {
  elements.sessionResult.hidden = true;
  elements.sessionError.hidden = true;
  elements.sessionError.textContent = "";
  elements.commandOutput.value = "";
}

export function openRonSheet() {
  const problems = configProblems();
  elements.ronProblems.hidden = problems.length === 0;
  elements.ronProblems.textContent = problems.join(" ");
  elements.ronOutput.textContent = generateRon();
  elements.ronDialog.showModal();
}

export function downloadRon() {
  if (lastDownloadUrl) {
    URL.revokeObjectURL(lastDownloadUrl);
  }
  lastDownloadUrl = URL.createObjectURL(
    new Blob([generateRon()], { type: "text/plain;charset=utf-8" })
  );
  const link = element("a");
  link.href = lastDownloadUrl;
  link.download = "config.ron";
  document.body.append(link);
  link.click();
  link.remove();
  announce("config.ron downloaded.");
}

export async function openInstallSheet() {
  elements.installDialog.showModal();
  await createInstallCommand();
}

async function createInstallCommand() {
  const ron = generateRon();
  elements.copyCommandButton.disabled = true;
  elements.sessionResult.hidden = true;
  elements.sessionError.hidden = true;

  try {
    const problems = configProblems();
    if (problems.length > 0) {
      throw new Error(problems.join(" "));
    }
    if (typeof CompressionStream !== "function") {
      throw new Error("This browser cannot compress the config. Download the RON instead.");
    }

    const code = await encodeConfigCode(ron);
    if (ron !== generateRon()) {
      throw new Error("The configuration changed while the command was being built. Build it again.");
    }

    elements.configSizeOutput.textContent = describeConfigSize(ron, code);
    elements.commandOutput.value = installCommand(code);
    elements.sessionResult.hidden = false;
    announce("Install command created.");
  } catch (error) {
    console.error("Could not build the install command.", error);
    elements.sessionError.textContent = error instanceof Error
      ? error.message
      : "Could not build the install command.";
    elements.sessionError.hidden = false;
    announce("Install command could not be created.");
  } finally {
    elements.copyCommandButton.disabled = false;
  }
}

async function encodeConfigCode(ron) {
  const compressed = new Blob([ron])
    .stream()
    .pipeThrough(new CompressionStream("gzip"));
  const bytes = new Uint8Array(await new Response(compressed).arrayBuffer());
  return base64Url(bytes);
}

function base64Url(bytes) {
  let binary = "";
  bytes.forEach((byte) => {
    binary += String.fromCharCode(byte);
  });
  return btoa(binary)
    .replaceAll("+", "-")
    .replaceAll("/", "_")
    .replaceAll("=", "");
}

function installCommand(code) {
  return `curl -fsSL ${installerUrl} | STATUSLINE_INSTALL_CONFIG=${code} sh`;
}

function describeConfigSize(ron, code) {
  const ronBytes = new TextEncoder().encode(ron).length;
  return `${ronBytes} B RON · ${code.length} char code`;
}

export async function copyInstallCommand() {
  const command = elements.commandOutput.value;
  if (!command) {
    return;
  }

  if (navigator.clipboard && globalThis.isSecureContext) {
    try {
      await navigator.clipboard.writeText(command);
      setCopyButtonState();
      return;
    } catch (error) {
      console.error("Clipboard API failed.", error);
    }
  }

  elements.commandOutput.focus();
  elements.commandOutput.select();
  document.execCommand("copy");
  setCopyButtonState();
}

function setCopyButtonState() {
  elements.copyCommandButton.textContent = "Copied";
  announce("Install command copied.");
  elements.copyCommandButton.addEventListener(
    "blur",
    () => {
      elements.copyCommandButton.textContent = "Copy";
    },
    { once: true }
  );
}
