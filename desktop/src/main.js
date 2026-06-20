const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;

// UI Elements
let activeTab = 'tab-translate';
let appConfig = null;
let serverPort = null;
let isServerRunning = false;

// DOM Cache
let navItems;
let tabContents;
let sourceInput;
let targetOutput;
let targetLangSelect;
let btnTranslate;
let btnClear;
let btnCopy;
let metricsContainer;
let metricSpeed;
let metricTime;

// Settings DOM Cache
let engineStatus;
let enginePort;
let engineApiUrl;
let btnCopyApi;
let modelFileStatus;
let btnToggleServer;
let downloaderPanel;
let btnStartDownload;
let downloadProgressBox;
let downloadStatusLabel;
let downloadSpeed;
let downloadProgressBar;
let downloadBytesInfo;
let downloadPercent;
let sidebarStatusDot;
let sidebarStatusText;

// Settings Fields
let cfgModelDir;
let btnBrowseDir;
let cfgPortMode;
let fixedPortBox;
let cfgPort;
let cfgAllowExternal;
let cfgThreads;
let cfgContextSize;
let cfgPromptTemplate;
let settingsForm;

// Initialize
window.addEventListener("DOMContentLoaded", async () => {
  initDOM();
  setupEventListeners();
  await loadSettings();
  await checkStatus();
  setupDownloadListener();
});

function initDOM() {
  navItems = document.querySelectorAll(".nav-item");
  tabContents = document.querySelectorAll(".tab-content");
  sourceInput = document.querySelector("#source-text-input");
  targetOutput = document.querySelector("#target-text-output");
  targetLangSelect = document.querySelector("#target-lang-select");
  btnTranslate = document.querySelector("#btn-translate");
  btnClear = document.querySelector("#btn-clear");
  btnCopy = document.querySelector("#btn-copy");
  metricsContainer = document.querySelector("#translation-metrics");
  metricSpeed = document.querySelector("#metric-speed");
  metricTime = document.querySelector("#metric-time");

  // Settings
  engineStatus = document.querySelector("#engine-status");
  enginePort = document.querySelector("#engine-port");
  engineApiUrl = document.querySelector("#engine-api-url");
  btnCopyApi = document.querySelector("#btn-copy-api");
  modelFileStatus = document.querySelector("#model-file-status");
  btnToggleServer = document.querySelector("#btn-toggle-server");
  downloaderPanel = document.querySelector("#downloader-panel");
  btnStartDownload = document.querySelector("#btn-start-download");
  downloadProgressBox = document.querySelector("#download-progress-box");
  downloadStatusLabel = document.querySelector("#download-status-label");
  downloadSpeed = document.querySelector("#download-speed");
  downloadProgressBar = document.querySelector("#download-progress-bar");
  downloadBytesInfo = document.querySelector("#download-bytes-info");
  downloadPercent = document.querySelector("#download-percent");
  sidebarStatusDot = document.querySelector("#sidebar-status-dot");
  sidebarStatusText = document.querySelector("#sidebar-status-text");

  // Form Fields
  cfgModelDir = document.querySelector("#cfg-model-dir");
  btnBrowseDir = document.querySelector("#btn-browse-dir");
  cfgPortMode = document.querySelector("#cfg-port-mode");
  fixedPortBox = document.querySelector("#fixed-port-box");
  cfgPort = document.querySelector("#cfg-port");
  cfgAllowExternal = document.querySelector("#cfg-allow-external");
  cfgThreads = document.querySelector("#cfg-threads");
  cfgContextSize = document.querySelector("#cfg-context-size");
  cfgPromptTemplate = document.querySelector("#cfg-prompt-template");
  settingsForm = document.querySelector("#settings-form");
}

function setupEventListeners() {
  // Tab Switcher
  navItems.forEach(item => {
    item.addEventListener("click", () => {
      const tabId = item.getAttribute("data-tab");
      switchTab(tabId);
    });
  });

  // Action Buttons
  btnClear.addEventListener("click", () => {
    sourceInput.value = "";
    targetOutput.textContent = "翻译结果将在这里流式展示...";
    targetOutput.classList.add("output-placeholder");
    metricsContainer.style.display = "none";
  });

  btnCopy.addEventListener("click", () => {
    if (targetOutput.classList.contains("output-placeholder")) return;
    const text = targetOutput.textContent;
    navigator.clipboard.writeText(text);
    showToast("复制成功");
  });

  btnCopyApi.addEventListener("click", () => {
    const text = engineApiUrl.textContent;
    navigator.clipboard.writeText(text);
    showToast("API 地址复制成功");
  });

  btnTranslate.addEventListener("click", doTranslate);

  // Folder Pick Dialog
  btnBrowseDir.addEventListener("click", async () => {
    try {
      const selected = await invoke("select_directory");
      if (selected) {
        cfgModelDir.value = selected;
      }
    } catch (e) {
      console.error(e);
      showToast("选择文件夹失败: " + e, true);
    }
  });

  // Port mode conditional fields
  cfgPortMode.addEventListener("change", () => {
    if (cfgPortMode.value === "fixed") {
      fixedPortBox.style.display = "block";
    } else {
      fixedPortBox.style.display = "none";
    }
  });

  // Save Settings Form
  settingsForm.addEventListener("submit", async (e) => {
    e.preventDefault();
    await saveSettings();
  });

  // Toggle Server
  btnToggleServer.addEventListener("click", toggleServer);

  // Start Download
  btnStartDownload.addEventListener("click", async () => {
    try {
      btnStartDownload.disabled = true;
      downloadProgressBox.style.display = "block";
      downloadStatusLabel.textContent = "启动下载器...";
      await invoke("download_model");
    } catch (e) {
      console.error(e);
      showToast("启动下载失败: " + e, true);
      btnStartDownload.disabled = false;
    }
  });
}

function switchTab(tabId) {
  navItems.forEach(item => {
    if (item.getAttribute("data-tab") === tabId) {
      item.classList.add("active");
    } else {
      item.classList.remove("active");
    }
  });

  tabContents.forEach(content => {
    if (content.id === tabId) {
      content.classList.add("active");
    } else {
      content.classList.remove("active");
    }
  });
  activeTab = tabId;
}

// Load configurations from Rust AppConfig
async function loadSettings() {
  try {
    appConfig = await invoke("get_config");
    cfgModelDir.value = appConfig.model_dir;
    cfgPortMode.value = appConfig.use_random_port ? "random" : "fixed";
    if (appConfig.use_random_port) {
      fixedPortBox.style.display = "none";
    } else {
      fixedPortBox.style.display = "block";
    }
    cfgPort.value = appConfig.port;
    cfgAllowExternal.checked = appConfig.allow_external;
    cfgThreads.value = appConfig.threads;
    cfgContextSize.value = appConfig.context_size;
    cfgPromptTemplate.value = appConfig.prompt_template;
  } catch (e) {
    console.error("加载设置失败", e);
    showToast("加载设置失败", true);
  }
}

// Save settings to Rust config.json
async function saveSettings() {
  try {
    const config = {
      port: parseInt(cfgPort.value, 10) || 8080,
      use_random_port: cfgPortMode.value === "random",
      model_dir: cfgModelDir.value,
      threads: parseInt(cfgThreads.value, 10) || 4,
      context_size: parseInt(cfgContextSize.value, 10) || 2048,
      allow_external: cfgAllowExternal.checked,
      prompt_template: cfgPromptTemplate.value
    };

    await invoke("set_config", { config });
    appConfig = config;
    showToast("配置保存成功！");
    await checkStatus(); // Recheck since path might have changed
  } catch (e) {
    console.error("保存设置失败", e);
    showToast("保存设置失败: " + e, true);
  }
}

// Check server status and model status
async function checkStatus() {
  try {
    // 1. Check Model GGUF File Status
    const modelExists = await invoke("check_model_status");
    if (modelExists) {
      modelFileStatus.textContent = "已检测到模型 (Ready)";
      modelFileStatus.className = "status-badge ok";
      downloaderPanel.style.display = "none";
      btnToggleServer.disabled = false;
    } else {
      modelFileStatus.textContent = "未检测到模型 (GGUF 缺失)";
      modelFileStatus.className = "status-badge error";
      downloaderPanel.style.display = "block";
      btnToggleServer.disabled = true;
    }

    // 2. Check Server Running Status
    const status = await invoke("check_server_status");
    isServerRunning = status.running;
    serverPort = status.port;

    if (isServerRunning) {
      engineStatus.textContent = "运行中";
      engineStatus.className = "status-badge running";
      enginePort.textContent = serverPort;
      engineApiUrl.textContent = `http://127.0.0.1:${serverPort}/v1`;
      btnToggleServer.textContent = "停止引擎";
      btnToggleServer.className = "btn btn-danger";

      sidebarStatusDot.className = "status-indicator-dot running";
      sidebarStatusText.textContent = `引擎在线 (端口 ${serverPort})`;
    } else {
      engineStatus.textContent = "已停止";
      engineStatus.className = "status-badge stopped";
      enginePort.textContent = "--";
      engineApiUrl.textContent = "http://127.0.0.1:--/v1";
      btnToggleServer.textContent = "启动引擎";
      btnToggleServer.className = "btn btn-success";

      sidebarStatusDot.className = "status-indicator-dot stopped";
      sidebarStatusText.textContent = "引擎未启动";
    }
  } catch (e) {
    console.error(e);
  }
}

// Toggle Start / Stop llama-server
async function toggleServer() {
  try {
    btnToggleServer.disabled = true;
    if (isServerRunning) {
      engineStatus.textContent = "停止中...";
      engineStatus.className = "status-badge loading";
      await invoke("stop_server");
      showToast("翻译服务已停止");
    } else {
      engineStatus.textContent = "启动中...";
      engineStatus.className = "status-badge loading";
      const port = await invoke("start_server");
      showToast(`翻译服务启动成功！运行于端口 ${port}`);
    }
  } catch (e) {
    console.error(e);
    showToast("服务操作失败: " + e, true);
  } finally {
    await checkStatus();
    btnToggleServer.disabled = false;
  }
}

// Setup background model downloader listener
function setupDownloadListener() {
  listen("download-progress", (event) => {
    const { percentage, speed, downloaded_bytes, total_bytes, finished, error } = event.payload;

    if (error) {
      downloadStatusLabel.textContent = "下载失败";
      downloadSpeed.textContent = "0.0 MB/s";
      showToast("模型下载失败: " + error, true);
      btnStartDownload.disabled = false;
      return;
    }

    if (finished) {
      downloadStatusLabel.textContent = "下载完成，正在加载模型...";
      downloadPercent.textContent = "100%";
      downloadProgressBar.style.style = "width: 100%";
      showToast("模型下载完成！");
      btnStartDownload.disabled = false;
      
      // Delay status recheck to let Rust rename the file
      setTimeout(async () => {
        await checkStatus();
      }, 1000);
      return;
    }

    // Update downloading panel UI
    downloadStatusLabel.textContent = "正在下载模型权重文件...";
    downloadPercent.textContent = `${percentage.toFixed(1)}%`;
    downloadProgressBar.style.width = `${percentage}%`;
    downloadSpeed.textContent = `${speed.toFixed(1)} MB/s`;

    const downloadedMB = (downloaded_bytes / 1024 / 1024).toFixed(1);
    const totalMB = (total_bytes / 1024 / 1024).toFixed(1);
    downloadBytesInfo.textContent = `${downloadedMB} MB / ${totalMB} MB`;
  });
}

// Stream Translation (Call completions API)
async function doTranslate() {
  const text = sourceInput.value.trim();
  if (!text) {
    showToast("请输入需要翻译的文本！", true);
    return;
  }

  if (!isServerRunning) {
    showToast("翻译引擎未启动，请先在系统设置中启动引擎！", true);
    switchTab('tab-settings');
    return;
  }

  // Pre-translation UI setups
  btnTranslate.disabled = true;
  metricsContainer.style.display = "flex";
  metricSpeed.textContent = "正在生成 Prompt...";
  metricTime.textContent = "0.0s";
  targetOutput.textContent = "翻译中，请稍候...";
  targetOutput.classList.remove("output-placeholder");

  const targetLang = targetLangSelect.value;
  let promptText = appConfig.prompt_template
    .replace("{target_lang}", targetLang)
    .replace("{source_text}", text);

  try {
    const response = await fetch(`http://127.0.0.1:${serverPort}/v1/chat/completions`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json'
      },
      body: JSON.stringify({
        messages: [{ role: 'user', content: promptText }],
        temperature: 0.3,
        top_p: 0.6,
        top_k: 20,
        repetition_penalty: 1.05,
        max_tokens: 2048,
        stream: true
      })
    });

    if (!response.ok) {
      throw new Error(`API returned HTTP error status: ${response.status}`);
    }

    const reader = response.body.getReader();
    const decoder = new TextDecoder("utf-8");
    let outputText = "";
    let tokenCount = 0;
    const startTime = performance.now();

    targetOutput.textContent = ""; // Clear placeholders

    while (true) {
      const { done, value } = await reader.read();
      if (done) break;

      const chunk = decoder.decode(value);
      const lines = chunk.split('\n');

      for (const line of lines) {
        if (line.startsWith('data: ')) {
          const dataStr = line.slice(6).trim();
          if (dataStr === '[DONE]') continue;

          try {
            const json = JSON.parse(dataStr);
            const content = json.choices[0].delta.content;
            if (content) {
              outputText += content;
              targetOutput.textContent = outputText;
              tokenCount++;

              const elapsed = (performance.now() - startTime) / 1000;
              metricSpeed.textContent = `${(tokenCount / elapsed).toFixed(1)} tokens/s`;
              metricTime.textContent = `${elapsed.toFixed(1)}s`;
            }
          } catch (e) {
            // Ignore JSON parse errors on incomplete chunk boundaries
          }
        }
      }
    }

    if (outputText === "") {
      targetOutput.textContent = "模型返回了空译文，请确认模型加载配置。";
      targetOutput.classList.add("output-placeholder");
    }

  } catch (e) {
    console.error(e);
    targetOutput.textContent = "发生错误，翻译失败: \n" + e.message;
    targetOutput.classList.add("output-placeholder");
    showToast("翻译失败: " + e.message, true);
  } finally {
    btnTranslate.disabled = false;
  }
}

// Elegant Native-like Notification Toast
function showToast(message, isError = false) {
  const toast = document.createElement("div");
  toast.className = "toast";
  if (isError) toast.classList.add("error");
  toast.textContent = message;

  // Add toast styling dynamically to prevent styling clutter
  Object.assign(toast.style, {
    position: "fixed",
    bottom: "32px",
    right: "32px",
    padding: "12px 24px",
    borderRadius: "10px",
    backgroundColor: isError ? "var(--color-error)" : "var(--bg-sidebar)",
    color: "#fff",
    border: "1px solid var(--border-color)",
    boxShadow: "0 10px 25px rgba(0,0,0,0.4)",
    fontSize: "14px",
    fontWeight: "500",
    zIndex: "9999",
    opacity: "0",
    transform: "translateY(20px)",
    transition: "all 0.3s cubic-bezier(0.16, 1, 0.3, 1)"
  });

  document.body.appendChild(toast);

  // Trigger entering animation
  setTimeout(() => {
    toast.style.opacity = "1";
    toast.style.transform = "translateY(0)";
  }, 50);

  // Disappear after 3s
  setTimeout(() => {
    toast.style.opacity = "0";
    toast.style.transform = "translateY(20px)";
    setTimeout(() => {
      document.body.removeChild(toast);
    }, 300);
  }, 3000);
}
