const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;
const appWindow = () => window.__TAURI__.window.getCurrentWindow();

const $ = (id) => document.getElementById(id);

// ---------- Orb ----------

let orb = null;
try {
  orb = new window.ByteLivingOrb($('byte-orb-canvas'));
} catch (err) {
  console.error('Failed to initialize orb:', err);
}

// ---------- Elements ----------

const presenceWidget = document.querySelector('.presence-widget');
const talkButton = $('talk-button');
const statusPill = $('status-pill');
const statusText = $('status-text');
const textToggle = $('text-toggle');
const settingsToggle = $('settings-toggle');

const chatPanel = $('chat-panel');
const textForm = $('text-form');
const textInput = $('text-input');
const textSend = $('text-send');
const textFeedback = $('text-feedback');
const textClose = $('text-close');

const settingsPanel = $('settings-panel');
const settingsForm = $('settings-form');
const settingsTitle = $('settings-title');
const settingsSubtitle = $('settings-subtitle');
const settingsClose = $('settings-close');
const settingsSave = $('settings-save');
const settingsFeedback = $('settings-feedback');
const ollamaUrlInput = $('ollama-url');
const checkConnectionBtn = $('check-connection');
const connStatus = $('conn-status');
const connMessage = $('conn-message');
const wakeToggle = $('wake-toggle');
const resetMemoryBtn = $('reset-memory');

const modelRows = {
  text: { row: $('row-text'), name: $('text-model-name'), model: '' },
  vision: { row: $('row-vision'), name: $('vision-model-name'), model: '' },
};

// ---------- State ----------

let isRunning = false;
let statusResetTimer = null;
let placementMode = 'center';
let manualPlacement = false;
let placementCheckPending = false;
let orbPointerStart = null;
let orbWasDragged = false;
let suppressNextOrbClick = false;
let setupComplete = true;
let pullsInFlight = 0;
let resetConfirmTimer = null;

const currentMode = () => (manualPlacement ? 'manual' : placementMode);
const isChatOpen = () => document.body.classList.contains('chat-open');
const isSettingsOpen = () => document.body.classList.contains('settings-open');

// ---------- Status ----------

const STATUS = {
  idle: { text: 'Ready', label: 'Start talking to Byte', title: 'Click to talk · Ctrl+B' },
  listening: { text: 'Listening', label: 'Stop listening', title: 'Click to stop' },
  thinking: { text: 'Thinking', label: 'Cancel response', title: 'Click to cancel' },
  speaking: { text: 'Speaking', label: 'Stop speaking', title: 'Click to stop' },
  cancelled: { text: 'Cancelled', label: 'Start talking to Byte', title: 'Click to talk · Ctrl+B' },
  error: { text: 'Something went wrong', label: 'Try again', title: 'Click to try again' },
};

function setStatus(state) {
  if (state !== 'error' && statusResetTimer) {
    clearTimeout(statusResetTimer);
    statusResetTimer = null;
  }
  if (state === 'hidden') {
    presenceWidget.classList.add('hidden');
    return;
  }
  presenceWidget.classList.remove('hidden');
  const s = STATUS[state] || STATUS.idle;
  statusPill.dataset.state = STATUS[state] ? state : 'idle';
  statusText.textContent = s.text;
  talkButton.setAttribute('aria-label', s.label);
  talkButton.title = s.title;
  orb?.setState(state);
}

// ---------- Window sizing ----------

async function resizeWindow(expanded, setupPanel) {
  await invoke('set_conversation_window_size', { expanded, placementMode: currentMode(), setupPanel });
}

// ---------- Chat panel ----------

async function setChatOpen(open) {
  if (open === isChatOpen()) return;
  if (open) {
    if (isSettingsOpen()) await setSettingsOpen(false);
    try {
      await resizeWindow(true, false);
      document.body.classList.add('chat-open');
      chatPanel.hidden = false;
      textToggle.setAttribute('aria-expanded', 'true');
      await appWindow().show();
      await appWindow().setFocus();
      textInput.focus();
    } catch (err) {
      console.error('Failed to open chat:', err);
    }
  } else {
    chatPanel.hidden = true;
    document.body.classList.remove('chat-open');
    textToggle.setAttribute('aria-expanded', 'false');
    try {
      await resizeWindow(false, false);
    } catch (err) {
      console.error('Failed to close chat:', err);
    }
  }
}

function autosizeInput() {
  textInput.style.height = 'auto';
  textInput.style.height = `${Math.min(textInput.scrollHeight, 90)}px`;
  textSend.disabled = isRunning || !textInput.value.trim();
}

textToggle.addEventListener('click', () => setChatOpen(!isChatOpen()));
textClose.addEventListener('click', () => setChatOpen(false));
textInput.addEventListener('input', autosizeInput);
textInput.addEventListener('keydown', (event) => {
  if (event.key === 'Enter' && !event.shiftKey) {
    event.preventDefault();
    textForm.requestSubmit();
  }
});

textForm.addEventListener('submit', async (event) => {
  event.preventDefault();
  const text = textInput.value.trim();
  if (!text || isRunning) return;

  textInput.value = '';
  autosizeInput();
  isRunning = true;
  textSend.disabled = true;
  try {
    const interaction = invoke('start_text_interaction', { text });
    await setChatOpen(false);
    await interaction;
    textFeedback.textContent = 'Byte replies out loud · Enter to send';
  } catch (err) {
    console.error('Text interaction failed:', err);
    textFeedback.textContent = String(err);
    setStatus('error');
    statusResetTimer = setTimeout(() => setStatus('idle'), 3500);
  } finally {
    isRunning = false;
    autosizeInput();
  }
});

// ---------- Settings panel ----------

function setFeedback(message, tone = '') {
  settingsFeedback.textContent = message;
  settingsFeedback.dataset.tone = tone;
}

function displayEndpoint(url) {
  return String(url || '').replace(/\/v1\/chat\/completions\/?$/, '').replace(/\/+$/, '');
}

function setConnState(state, message) {
  connStatus.dataset.state = state;
  connMessage.textContent = message;
}

function setModelRow(key, state, label) {
  const { row } = modelRows[key];
  const badge = row.querySelector('.badge');
  const pull = row.querySelector('[data-pull]');
  badge.dataset.state = state;
  badge.textContent = label;
  pull.hidden = state !== 'missing';
}

async function checkModelStatus() {
  const url = ollamaUrlInput.value.trim();
  if (!url) {
    setConnState('bad', 'Enter your Ollama endpoint.');
    return;
  }
  checkConnectionBtn.disabled = true;
  setConnState('busy', 'Checking Ollama…');
  for (const key of Object.keys(modelRows)) {
    if (modelRows[key].row.querySelector('.badge').dataset.state !== 'busy') setModelRow(key, 'unknown', 'Checking');
  }
  try {
    const status = await invoke('get_model_status', { ollamaUrl: url });
    applyModelStatus(status);
  } catch (err) {
    setConnState('bad', String(err));
  } finally {
    checkConnectionBtn.disabled = false;
  }
}

function applyModelStatus(status) {
  if (!status.reachable) {
    setConnState('bad', status.message);
    setModelRow('text', 'offline', 'Offline');
    setModelRow('vision', 'offline', 'Offline');
    return;
  }
  setConnState(status.textInstalled && status.visionInstalled ? 'ok' : 'warn', status.message);
  for (const [key, installed] of [['text', status.textInstalled], ['vision', status.visionInstalled]]) {
    if (modelRows[key].row.querySelector('.badge').dataset.state === 'busy') continue;
    setModelRow(key, installed ? 'ready' : 'missing', installed ? 'Ready' : 'Not installed');
  }
}

async function pullModel(key) {
  const { model } = modelRows[key];
  setModelRow(key, 'busy', 'Downloading');
  pullsInFlight += 1;
  setFeedback(`Downloading ${model}. This can take a few minutes…`);
  try {
    const status = await invoke('pull_ollama_model', { ollamaUrl: ollamaUrlInput.value.trim(), model });
    setModelRow(key, 'unknown', 'Checking');
    applyModelStatus(status);
    setFeedback(`${model} is ready.`, 'ok');
  } catch (err) {
    setModelRow(key, 'missing', 'Not installed');
    setFeedback(String(err), 'error');
  } finally {
    pullsInFlight -= 1;
  }
}

for (const key of Object.keys(modelRows)) {
  modelRows[key].row.querySelector('[data-pull]').addEventListener('click', () => pullModel(key));
}

function renderWakeToggle(enabled) {
  wakeToggle.setAttribute('aria-checked', String(enabled));
  wakeToggle.title = enabled ? 'Wake word is on' : 'Wake word is off';
}

async function loadSettings() {
  const settings = await invoke('get_model_settings');
  setupComplete = settings.setupComplete;
  modelRows.text.model = settings.textModel;
  modelRows.vision.model = settings.visionModel;
  modelRows.text.name.textContent = settings.textModel;
  modelRows.vision.name.textContent = settings.visionModel;
  ollamaUrlInput.value = displayEndpoint(settings.ollamaUrl);

  settingsTitle.textContent = setupComplete ? 'Settings' : 'Welcome to Byte';
  settingsSubtitle.textContent = setupComplete
    ? 'Byte runs fully on this device.'
    : 'Make sure Ollama is running and both models are installed.';
  settingsSave.textContent = setupComplete ? 'Save' : 'Get started';
  if (!pullsInFlight) setFeedback('');

  invoke('get_wake_word_enabled')
    .then((enabled) => {
      renderWakeToggle(enabled);
      wakeToggle.disabled = false;
    })
    .catch(() => {
      wakeToggle.disabled = true;
      wakeToggle.title = 'Wake word setting unavailable';
    });

  checkModelStatus();
}

async function setSettingsOpen(open) {
  if (open === isSettingsOpen()) return;
  if (open) {
    if (isChatOpen()) await setChatOpen(false);
    try {
      await resizeWindow(true, true);
      document.body.classList.add('settings-open');
      settingsPanel.hidden = false;
      settingsToggle.setAttribute('aria-expanded', 'true');
      await appWindow().show();
      await appWindow().setFocus();
      await loadSettings();
      ollamaUrlInput.focus();
    } catch (err) {
      console.error('Failed to open settings:', err);
      setFeedback(String(err), 'error');
    }
  } else {
    settingsPanel.hidden = true;
    document.body.classList.remove('settings-open');
    settingsToggle.setAttribute('aria-expanded', 'false');
    cancelResetConfirm();
    try {
      await resizeWindow(false, true);
    } catch (err) {
      console.error('Failed to close settings:', err);
    }
  }
}

settingsToggle.addEventListener('click', () => setSettingsOpen(!isSettingsOpen()));
settingsClose.addEventListener('click', () => setSettingsOpen(false));
checkConnectionBtn.addEventListener('click', checkModelStatus);
ollamaUrlInput.addEventListener('keydown', (event) => {
  if (event.key === 'Enter') {
    event.preventDefault();
    settingsForm.requestSubmit();
  }
});

settingsForm.addEventListener('submit', async (event) => {
  event.preventDefault();
  const url = ollamaUrlInput.value.trim();
  if (!/^https?:\/\/\S+$/.test(url)) {
    setFeedback('Enter a valid endpoint, such as http://localhost:11434.', 'error');
    ollamaUrlInput.focus();
    return;
  }
  settingsSave.disabled = true;
  setFeedback('Saving…');
  try {
    const wasFirstRun = !setupComplete;
    const saved = await invoke('save_model_settings', { ollamaUrl: url });
    setupComplete = saved.setupComplete;
    ollamaUrlInput.value = displayEndpoint(saved.ollamaUrl);
    if (wasFirstRun) {
      await setSettingsOpen(false);
      return;
    }
    settingsTitle.textContent = 'Settings';
    settingsSave.textContent = 'Save';
    setFeedback('Saved.', 'ok');
    checkModelStatus();
  } catch (err) {
    setFeedback(String(err), 'error');
  } finally {
    settingsSave.disabled = false;
  }
});

wakeToggle.addEventListener('click', async () => {
  const enabled = wakeToggle.getAttribute('aria-checked') !== 'true';
  wakeToggle.disabled = true;
  renderWakeToggle(enabled);
  try {
    renderWakeToggle(await invoke('set_wake_word_enabled', { enabled }));
  } catch (err) {
    renderWakeToggle(!enabled);
    setFeedback(`Couldn't change wake word: ${err}`, 'error');
  } finally {
    wakeToggle.disabled = false;
  }
});

function cancelResetConfirm() {
  clearTimeout(resetConfirmTimer);
  resetConfirmTimer = null;
  resetMemoryBtn.classList.remove('confirming');
  resetMemoryBtn.textContent = 'Reset';
}

resetMemoryBtn.addEventListener('click', async () => {
  if (!resetConfirmTimer) {
    resetMemoryBtn.classList.add('confirming');
    resetMemoryBtn.textContent = 'Confirm';
    resetConfirmTimer = setTimeout(cancelResetConfirm, 4000);
    return;
  }
  cancelResetConfirm();
  resetMemoryBtn.disabled = true;
  try {
    await invoke('reset_saved_memory');
    setFeedback('Saved memory cleared.', 'ok');
  } catch (err) {
    setFeedback(String(err), 'error');
  } finally {
    resetMemoryBtn.disabled = false;
  }
});

// ---------- Voice interaction ----------

async function runInteraction() {
  if (isRunning) return;
  if (statusResetTimer) {
    clearTimeout(statusResetTimer);
    statusResetTimer = null;
  }
  isRunning = true;

  try {
    await appWindow().show();
    await appWindow().setFocus();
  } catch (err) {
    console.warn('Window focus notice:', err);
  }

  let result = null;
  let failed = false;
  try {
    result = await invoke('start_interaction');
    if (result?.auto_listen) {
      setTimeout(() => {
        isRunning = false;
        runInteraction();
      }, 400);
      return;
    }
  } catch (err) {
    console.error('Interaction error:', err);
    failed = true;
  } finally {
    if (!result?.auto_listen) {
      isRunning = false;
      orb?.setAudioLevel(0);
      if (failed) {
        setStatus('error');
        statusResetTimer = setTimeout(() => setStatus('idle'), 3500);
      } else {
        setStatus('idle');
      }
    }
  }
}

async function toggleInteraction() {
  if (!isRunning) {
    runInteraction();
    return;
  }
  try {
    await invoke('stop_action');
  } catch (err) {
    console.error('Failed to stop interaction:', err);
  }
}

// Orb: click to talk, drag to move the window
talkButton.addEventListener('pointerdown', (event) => {
  if (event.button !== 0) return;
  orbPointerStart = { x: event.screenX, y: event.screenY };
  orbWasDragged = false;
});

window.addEventListener('pointermove', async (event) => {
  if (!orbPointerStart || orbWasDragged) return;
  if (Math.hypot(event.screenX - orbPointerStart.x, event.screenY - orbPointerStart.y) < 7) return;
  orbWasDragged = true;
  manualPlacement = true;
  try {
    await appWindow().startDragging();
  } catch (err) {
    console.error('Could not drag window:', err);
    manualPlacement = false;
    orbWasDragged = false;
    orbPointerStart = null;
  }
});

window.addEventListener('pointerup', () => {
  if (orbWasDragged) {
    suppressNextOrbClick = true;
    setTimeout(() => { suppressNextOrbClick = false; }, 600);
  }
  orbPointerStart = null;
  orbWasDragged = false;
});

window.addEventListener('pointercancel', () => {
  orbPointerStart = null;
  orbWasDragged = false;
});

talkButton.addEventListener('click', () => {
  if (suppressNextOrbClick) {
    suppressNextOrbClick = false;
    return;
  }
  toggleInteraction();
});

// ---------- Backend events ----------

listen('presence_state_changed', (event) => setStatus(event.payload));
listen('audio_level', (event) => orb?.setAudioLevel(typeof event.payload === 'number' ? event.payload : 0));
listen('wakeup', () => runInteraction());
listen('open_settings', () => setSettingsOpen(true));
listen('wake_word_changed', (event) => renderWakeToggle(Boolean(event.payload)));

// ---------- Keyboard ----------

window.addEventListener('keydown', async (event) => {
  if (event.key === 'Escape') {
    event.preventDefault();
    if (isSettingsOpen()) return setSettingsOpen(false);
    if (isChatOpen() && !isRunning) return setChatOpen(false);
    try {
      await invoke('stop_action');
    } catch (err) {
      console.error('Escape stop error:', err);
    }
  } else if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === 'b') {
    event.preventDefault();
    toggleInteraction();
  }
});

// ---------- Placement ----------

async function updatePresencePlacement() {
  if (manualPlacement || isRunning || isChatOpen() || isSettingsOpen() || placementCheckPending) return;
  placementCheckPending = true;
  try {
    placementMode = await invoke('reposition_presence', { currentMode: placementMode });
  } catch (err) {
    console.error('Could not position Byte:', err);
  } finally {
    placementCheckPending = false;
  }
}

setInterval(updatePresencePlacement, 2500);

// ---------- Startup ----------

setStatus('idle');
autosizeInput();

window.addEventListener('DOMContentLoaded', async () => {
  updatePresencePlacement();
  try {
    const settings = await invoke('get_model_settings');
    if (!settings.setupComplete) await setSettingsOpen(true);
  } catch (err) {
    console.error('Could not load setup state:', err);
  }
});
