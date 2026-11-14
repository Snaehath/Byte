const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;

let livingOrb = null;
try {
  const canvas = document.getElementById('byte-orb-canvas');
  const OrbClass = window.ByteLivingOrb || (typeof ByteLivingOrb !== 'undefined' ? ByteLivingOrb : null);
  if (OrbClass && canvas) {
    livingOrb = new OrbClass(canvas);
  } else {
    console.error('ByteLivingOrb class not found!');
  }
} catch (err) {
  console.error('Failed to initialize ByteLivingOrb:', err);
}

const statusText = document.getElementById('status-text');
const statusBadge = document.querySelector('.status-badge');
const presenceWidget = document.querySelector('.presence-widget');
const talkButton = document.getElementById('talk-button');
const wakeToggle = document.getElementById('wake-toggle');
const textToggle = document.getElementById('text-toggle');
const modelSettingsToggle = document.getElementById('model-settings-toggle');
const textClose = document.getElementById('text-close');
const conversationPanel = document.getElementById('conversation-panel');
const textForm = document.getElementById('text-form');
const textInput = document.getElementById('text-input');
const textSend = document.getElementById('text-send');
const textFeedback = document.getElementById('text-feedback');
const modelSetupPanel = document.getElementById('model-setup-panel');
const modelSetupForm = document.getElementById('model-setup-form');
const modelSetupClose = document.getElementById('model-setup-close');
const modelSetupTitle = document.getElementById('model-setup-title');
const modelProvider = document.getElementById('model-provider');
const localModelFields = document.getElementById('local-model-fields');
const cloudModelFields = document.getElementById('cloud-model-fields');
const modelPrivacy = document.getElementById('model-privacy');
const modelSetupFeedback = document.getElementById('model-setup-feedback');
const modelSetupSave = document.getElementById('model-setup-save');
const cloudKeyInput = document.getElementById('cloud-api-key');
const cloudKeyNote = document.getElementById('cloud-key-note');
const sameLocalModel = document.getElementById('use-same-model');
const sameCloudModel = document.getElementById('cloud-use-same-model');
const localVisionField = document.getElementById('local-vision-field');
const sameModelNote = document.getElementById('same-model-note');
const cloudSameModelNote = document.getElementById('cloud-same-model-note');
const ollamaModelNote = document.getElementById('ollama-model-note');
const ollamaRefreshButton = document.getElementById('ollama-refresh-models');
const ollamaPullButton = document.getElementById('ollama-pull-model');
const ollamaModelDownload = document.getElementById('ollama-model-download');

const modelInputs = {
  ollamaUrl: document.getElementById('ollama-url'),
  ollamaModel: document.getElementById('ollama-model'),
  visionModel: document.getElementById('vision-model'),
  cloudUrl: document.getElementById('cloud-url'),
  cloudModel: document.getElementById('cloud-model'),
  cloudVisionModel: document.getElementById('cloud-vision-model'),
};

let isRunning = false;
let statusResetTimer = null;
let placementMode = 'center';
let manualPlacement = false;
let orbPointerStart = null;
let orbWasDragged = false;
let suppressNextOrbClick = false;
let placementCheckPending = false;

function updateStatusBadge(state) {
  if (state !== 'error' && statusResetTimer) {
    clearTimeout(statusResetTimer);
    statusResetTimer = null;
  }
  statusBadge.className = `status-badge ${state}`;
  switch (state) {
    case 'idle':
      statusText.innerText = '';
      talkButton.setAttribute('aria-label', 'Start talking to Byte');
      talkButton.title = 'Click to talk · Ctrl+B';
      break;
    case 'listening':
      statusText.innerText = 'Listening';
      talkButton.setAttribute('aria-label', 'Stop listening to Byte');
      talkButton.title = 'Click to stop';
      break;
    case 'thinking':
      statusText.innerText = 'Thinking';
      talkButton.setAttribute('aria-label', 'Cancel Byte’s response');
      talkButton.title = 'Click to cancel';
      break;
    case 'speaking':
      statusText.innerText = 'Speaking';
      talkButton.setAttribute('aria-label', 'Stop Byte speaking');
      talkButton.title = 'Click to stop';
      break;
    case 'cancelled':
      statusText.innerText = 'Cancelled';
      talkButton.setAttribute('aria-label', 'Start talking to Byte');
      talkButton.title = 'Click to talk · Ctrl+B';
      break;
    case 'error':
      statusText.innerText = 'Couldn’t complete · click to retry';
      talkButton.setAttribute('aria-label', 'Retry talking to Byte');
      talkButton.title = 'Click to try again';
      break;
    case 'hidden':
      presenceWidget.classList.add('hidden');
      return;
  }
  presenceWidget.classList.remove('hidden');
}

function updateWakeToggle(enabled) {
  wakeToggle.textContent = enabled ? '◉' : '○';
  wakeToggle.setAttribute('aria-pressed', String(enabled));
  wakeToggle.setAttribute('aria-busy', 'false');
  wakeToggle.setAttribute(
    'aria-label',
    `Wake word detection is ${enabled ? 'on' : 'off'}. Click to turn it ${enabled ? 'off' : 'on'}.`
  );
  wakeToggle.title = `Wake word ${enabled ? 'on' : 'off'} · click to turn ${enabled ? 'off' : 'on'}`;
}

async function setConversationOpen(open) {
  const win = window.__TAURI__.window.getCurrentWindow();
  if (open) {
    conversationPanel.hidden = false;
    document.body.classList.add('text-open');
    textToggle.setAttribute('aria-expanded', 'true');
    try {
      await win.show();
      await invoke('set_conversation_window_size', { expanded: true, placementMode: manualPlacement ? 'manual' : placementMode, setupPanel: false });
      await win.setFocus();
      textInput.focus();
    } catch (err) {
      console.error('Failed to open conversation panel:', err);
      conversationPanel.hidden = true;
      document.body.classList.remove('text-open');
      textToggle.setAttribute('aria-expanded', 'false');
    }
  } else {
    conversationPanel.hidden = true;
    document.body.classList.remove('text-open');
    textToggle.setAttribute('aria-expanded', 'false');
    try {
      await invoke('set_conversation_window_size', { expanded: false, placementMode: manualPlacement ? 'manual' : placementMode, setupPanel: false });
    } catch (err) {
      console.error('Failed to close conversation panel:', err);
    }
  }
}

textToggle.addEventListener('click', async () => {
  if (!modelSetupPanel.hidden) await setModelSetupOpen(false);
  await setConversationOpen(true);
});
textClose.addEventListener('click', () => setConversationOpen(false));

function updateProviderFields() {
  const isCloud = modelProvider.value === 'cloud';
  localModelFields.hidden = isCloud;
  cloudModelFields.hidden = !isCloud;
  modelInputs.ollamaUrl.required = !isCloud;
  modelInputs.ollamaModel.required = !isCloud;
  modelInputs.cloudUrl.required = isCloud;
  modelInputs.cloudModel.required = isCloud;
  if (isCloud) sameCloudModel.checked = sameLocalModel.checked;
  else sameLocalModel.checked = sameCloudModel.checked;
  localVisionField.hidden = sameLocalModel.checked;
  modelInputs.cloudVisionModel.required = false;
  sameModelNote.hidden = isCloud || !sameLocalModel.checked;
  cloudSameModelNote.hidden = !isCloud || !sameCloudModel.checked;
  sameModelNote.textContent = 'Choose an installed vision-capable Ollama model, such as ministral-3:8b.';
  modelPrivacy.textContent = isCloud
    ? 'Requests go to your chosen cloud API. Your key is stored in Windows Credential Manager.'
    : 'Requests stay on this device and use your local Ollama service.';
}

function renderOllamaModels(models, preferred = modelInputs.ollamaModel.value) {
  const options = models.map((name) => {
    const option = document.createElement('option');
    option.value = name;
    option.textContent = name;
    return option;
  });
  if (preferred && !models.includes(preferred)) {
    const configured = document.createElement('option');
    configured.value = preferred;
    configured.textContent = `${preferred} (not detected)`;
    options.unshift(configured);
  }
  if (!options.length) {
    const empty = document.createElement('option');
    empty.value = '';
    empty.textContent = 'No installed models found';
    options.push(empty);
  }
  modelInputs.ollamaModel.replaceChildren(...options);
  modelInputs.ollamaModel.value = preferred && options.some((option) => option.value === preferred)
    ? preferred
    : models[0] || '';
  ollamaModelNote.textContent = models.length
    ? `${models.length} installed model${models.length === 1 ? '' : 's'} found. Select one above.`
    : 'No models installed yet. Enter a model name below to download one.';
}

async function loadModelSettings() {
  const settings = await invoke('get_model_settings');
  modelProvider.value = settings.provider;
  modelInputs.ollamaUrl.value = settings.ollamaUrl;
  modelInputs.ollamaModel.replaceChildren();
  modelInputs.visionModel.value = settings.visionModel;
  modelInputs.cloudUrl.value = settings.cloudUrl;
  modelInputs.cloudModel.value = settings.cloudModel;
  modelInputs.cloudVisionModel.value = settings.cloudVisionModel;
  sameLocalModel.checked = settings.useSameModel;
  sameCloudModel.checked = settings.useSameModel;
  cloudKeyInput.value = '';
  cloudKeyInput.placeholder = settings.apiKeyConfigured ? 'Saved key · leave blank to keep' : 'Paste your provider API key';
  cloudKeyNote.textContent = settings.apiKeyConfigured ? 'A key is saved securely on this PC.' : '';
  modelSetupTitle.textContent = settings.setupComplete ? 'Model settings' : 'Choose how Byte works';
  modelSetupSave.textContent = settings.setupComplete ? 'Save' : 'Get started';
  updateProviderFields();
  if (settings.provider === 'ollama') await refreshOllamaModels(settings.ollamaModel);
  return settings;
}

async function refreshOllamaModels(preferred = modelInputs.ollamaModel.value) {
  ollamaRefreshButton.disabled = true;
  ollamaModelNote.textContent = 'Checking Ollama…';
  try {
    const models = await invoke('list_ollama_models', { ollamaUrl: modelInputs.ollamaUrl.value });
    renderOllamaModels(models, preferred);
  } catch (err) {
    renderOllamaModels([], preferred);
    ollamaModelNote.textContent = `${err} Enter a model name below to download one.`;
  } finally {
    ollamaRefreshButton.disabled = false;
  }
}

async function setModelSetupOpen(open) {
  const win = window.__TAURI__.window.getCurrentWindow();
  if (open) {
    if (document.body.classList.contains('text-open')) await setConversationOpen(false);
    modelSetupPanel.hidden = false;
    document.body.classList.add('model-setup-open');
    try {
      await win.show();
      await invoke('set_conversation_window_size', { expanded: true, placementMode: manualPlacement ? 'manual' : placementMode, setupPanel: true });
      await win.setFocus();
      await loadModelSettings();
      modelProvider.focus();
    } catch (err) {
      console.error('Could not open model setup:', err);
      modelSetupFeedback.textContent = String(err);
    }
  } else {
    modelSetupPanel.hidden = true;
    document.body.classList.remove('model-setup-open');
    try {
      await invoke('set_conversation_window_size', { expanded: false, placementMode: manualPlacement ? 'manual' : placementMode, setupPanel: true });
    } catch (err) {
      console.error('Could not close model setup:', err);
    }
  }
}

modelProvider.addEventListener('change', () => {
  updateProviderFields();
  if (modelProvider.value === 'ollama') refreshOllamaModels(modelInputs.ollamaModel.value);
});
sameLocalModel.addEventListener('change', () => {
  sameCloudModel.checked = sameLocalModel.checked;
  updateProviderFields();
});
sameCloudModel.addEventListener('change', () => {
  sameLocalModel.checked = sameCloudModel.checked;
  updateProviderFields();
});
ollamaRefreshButton.addEventListener('click', refreshOllamaModels);
ollamaPullButton.addEventListener('click', async () => {
  const model = ollamaModelDownload.value.trim();
  if (!model) {
    ollamaModelNote.textContent = 'Enter a model name below, such as ministral-3:8b.';
    ollamaModelDownload.focus();
    return;
  }
  ollamaPullButton.disabled = true;
  ollamaRefreshButton.disabled = true;
  ollamaModelNote.textContent = `Downloading ${model}. Model downloads can be several GB and may take a while…`;
  try {
    const models = await invoke('pull_ollama_model', {
      ollamaUrl: modelInputs.ollamaUrl.value,
      model,
    });
    renderOllamaModels(models, model);
    ollamaModelDownload.value = '';
    ollamaModelNote.textContent = `${model} is installed and selected.`;
  } catch (err) {
    ollamaModelNote.textContent = String(err);
  } finally {
    ollamaPullButton.disabled = false;
    ollamaRefreshButton.disabled = false;
  }
});
modelSettingsToggle.addEventListener('click', () => setModelSetupOpen(true));
modelSetupClose.addEventListener('click', () => setModelSetupOpen(false));
modelSetupForm.addEventListener('submit', async (event) => {
  event.preventDefault();
  modelSetupSave.disabled = true;
  modelSetupFeedback.textContent = 'Saving…';
  try {
    await invoke('save_model_settings', {
      settings: {
        provider: modelProvider.value,
        ollamaUrl: modelInputs.ollamaUrl.value,
        ollamaModel: modelInputs.ollamaModel.value,
        visionModel: modelInputs.visionModel.value,
        useSameModel: modelProvider.value === 'cloud' ? sameCloudModel.checked : sameLocalModel.checked,
        cloudUrl: modelInputs.cloudUrl.value,
        cloudModel: modelInputs.cloudModel.value,
        cloudVisionModel: modelInputs.cloudVisionModel.value,
        apiKeyConfigured: false,
        setupComplete: true,
      },
      apiKey: cloudKeyInput.value,
    });
    modelSetupFeedback.textContent = '';
    await setModelSetupOpen(false);
  } catch (err) {
    modelSetupFeedback.textContent = String(err);
  } finally {
    modelSetupSave.disabled = false;
  }
});

textForm.addEventListener('submit', async (event) => {
  event.preventDefault();
  const text = textInput.value.trim();
  if (!text || isRunning) return;

  textInput.value = '';
  textSend.disabled = true;
  textFeedback.textContent = 'Thinking…';
  isRunning = true;
  try {
    const interaction = invoke('start_text_interaction', { text });
    await setConversationOpen(false);
    await interaction;
    textFeedback.textContent = 'Byte has replied aloud.';
  } catch (err) {
    console.error('Text interaction failed:', err);
    textFeedback.textContent = String(err);
  } finally {
    isRunning = false;
    textSend.disabled = false;
  }
});

textInput.addEventListener('keydown', (event) => {
  if (event.key === 'Enter' && !event.shiftKey) {
    event.preventDefault();
    textForm.requestSubmit();
  }
});

wakeToggle.addEventListener('click', async () => {
  const enabled = wakeToggle.getAttribute('aria-pressed') !== 'true';
  wakeToggle.disabled = true;
  try {
    updateWakeToggle(await invoke('set_wake_word_enabled', { enabled }));
  } catch (err) {
    console.error('Failed to change wake word setting:', err);
    wakeToggle.textContent = '◉';
    wakeToggle.title = 'Could not change wake word setting';
    wakeToggle.setAttribute('aria-label', 'Could not change wake word setting');
  } finally {
    wakeToggle.disabled = false;
  }
});

invoke('get_wake_word_enabled')
  .then((enabled) => {
    updateWakeToggle(enabled);
    wakeToggle.disabled = false;
  })
  .catch((err) => {
    console.error('Failed to load wake word setting:', err);
    wakeToggle.textContent = '◉';
    wakeToggle.title = 'Wake word setting unavailable';
    wakeToggle.setAttribute('aria-label', 'Wake word setting unavailable');
    wakeToggle.disabled = true;
  });

// Full conversational lifecycle run
async function runInteraction() {
  if (isRunning) return;
  if (statusResetTimer) {
    clearTimeout(statusResetTimer);
    statusResetTimer = null;
  }
  isRunning = true;

  try {
    const { getCurrentWindow } = window.__TAURI__.window;
    const win = getCurrentWindow();
    await win.show();
    await win.setFocus();
  } catch (err) {
    console.warn('Window focus/show notice:', err);
  }

  let result = null;
  let failed = false;
  try {
    result = await invoke('start_interaction');
    console.log('Interaction turn completed:', result);
    if (result) {
    }

    if (result && result.auto_listen) {
      setTimeout(() => {
        isRunning = false;
        runInteraction();
      }, 400);
      return;
    }
  } catch (err) {
    console.error('Interaction loop error:', err);
    failed = true;
  } finally {
    isRunning = false;
    if (livingOrb) livingOrb.setAudioLevel(0.0);
    if (!result || !result.auto_listen) {
      if (failed) {
        if (livingOrb) livingOrb.setState('idle');
        updateStatusBadge('error');
        statusResetTimer = setTimeout(() => updateStatusBadge('idle'), 3000);
      } else {
        if (livingOrb) livingOrb.setState('idle');
        updateStatusBadge('idle');
      }
    }
  }
}

// Click orb to speak or stop
talkButton.addEventListener('pointerdown', (event) => {
  if (event.button !== 0) return;
  orbPointerStart = { x: event.screenX, y: event.screenY };
  orbWasDragged = false;
});

window.addEventListener('pointermove', async (event) => {
  if (!orbPointerStart || orbWasDragged) return;
  const dx = event.screenX - orbPointerStart.x;
  const dy = event.screenY - orbPointerStart.y;
  if (Math.hypot(dx, dy) < 7) return;

  orbWasDragged = true;
  manualPlacement = true;
  try {
    await window.__TAURI__.window.getCurrentWindow().startDragging();
  } catch (err) {
    console.error('Could not drag Byte window:', err);
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

talkButton.addEventListener('click', async () => {
  if (suppressNextOrbClick) {
    suppressNextOrbClick = false;
    return;
  }
  if (!isRunning) {
    runInteraction();
  } else {
    try {
      await invoke('stop_action');
    } catch (err) {
      console.error('Failed to cancel interaction on orb click:', err);
    }
  }
});

// Tauri Event Subscriptions
listen('presence_state_changed', (event) => {
  const state = event.payload;
  if (livingOrb) livingOrb.setState(state);
  updateStatusBadge(state);
});

listen('audio_level', (event) => {
  const level = typeof event.payload === 'number' ? event.payload : 0.0;
  if (livingOrb) livingOrb.setAudioLevel(level);
});

listen('wakeup', () => {
  console.log('Byte wakeup signal received!');
  runInteraction();
});

// Keyboard controls: Escape to dismiss/cancel, Ctrl+B toggle
window.addEventListener('keydown', async (event) => {
  if (event.key === 'Escape') {
    event.preventDefault();
    if (document.body.classList.contains('text-open') && !isRunning) {
      await setConversationOpen(false);
      return;
    }
    try {
      await invoke('stop_action');
    } catch (err) {
      console.error('Escape stop error:', err);
    }
  } else if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === 'b') {
    event.preventDefault();
    if (!isRunning) {
      runInteraction();
    } else {
      await invoke('stop_action');
    }
  }
});

async function updatePresencePlacement() {
  if (manualPlacement || isRunning || document.body.classList.contains('text-open') || document.body.classList.contains('model-setup-open') || placementCheckPending) return;
  placementCheckPending = true;
  try {
    placementMode = await invoke('reposition_presence', { currentMode: placementMode });
  } catch (err) {
    console.error('Could not position Byte:', err);
  } finally {
    placementCheckPending = false;
  }
}

window.addEventListener('DOMContentLoaded', async () => {
  updatePresencePlacement();
  try {
    const settings = await invoke('get_model_settings');
    if (!settings.setupComplete) await setModelSetupOpen(true);
  } catch (err) {
    console.error('Could not load model setup state:', err);
  }
});

setInterval(updatePresencePlacement, 2500);

// Initial state setup
updateStatusBadge('idle');
if (livingOrb) livingOrb.setState('idle');
