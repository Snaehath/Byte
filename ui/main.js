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
const textClose = document.getElementById('text-close');
const conversationPanel = document.getElementById('conversation-panel');
const conversationMessages = document.getElementById('conversation-messages');
const textForm = document.getElementById('text-form');
const textInput = document.getElementById('text-input');
const textSend = document.getElementById('text-send');
const chatTab = document.getElementById('chat-tab');
const setupTab = document.getElementById('setup-tab');
const chatView = document.getElementById('chat-view');
const setupView = document.getElementById('setup-view');
const healthList = document.getElementById('health-list');
const healthRefresh = document.getElementById('health-refresh');
const settingsForm = document.getElementById('settings-form');
const ollamaUrlInput = document.getElementById('ollama-url');
const ollamaModelInput = document.getElementById('ollama-model');
const visionModelInput = document.getElementById('vision-model');
const whisperExeInput = document.getElementById('whisper-exe');
const whisperModelInput = document.getElementById('whisper-model');
const piperExeInput = document.getElementById('piper-exe');
const piperModelInput = document.getElementById('piper-model');
const settingsFeedback = document.getElementById('settings-feedback');
const settingsSave = document.getElementById('settings-save');
const memorySummary = document.getElementById('memory-summary');
const memoryRefresh = document.getElementById('memory-refresh');
const clearHistoryButton = document.getElementById('clear-history');
const resetMemoryButton = document.getElementById('reset-memory');

let isRunning = false;
let statusResetTimer = null;

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
  wakeToggle.textContent = `Wake word: ${enabled ? 'On' : 'Off'}`;
  wakeToggle.setAttribute('aria-pressed', String(enabled));
  wakeToggle.setAttribute('aria-busy', 'false');
  wakeToggle.setAttribute(
    'aria-label',
    `Wake word detection is ${enabled ? 'on' : 'off'}. Click to turn it ${enabled ? 'off' : 'on'}.`
  );
  wakeToggle.title = `Click to turn wake word detection ${enabled ? 'off' : 'on'}`;
}

function appendMessage(role, text) {
  if (!text || !text.trim()) return;
  const message = document.createElement('p');
  message.className = `conversation-message ${role}`;
  message.textContent = text.trim();
  conversationMessages.appendChild(message);
  while (conversationMessages.children.length > 20) {
    conversationMessages.firstElementChild.remove();
  }
  conversationMessages.scrollTop = conversationMessages.scrollHeight;
}

async function setConversationOpen(open) {
  const win = window.__TAURI__.window.getCurrentWindow();
  if (open) {
    conversationPanel.hidden = false;
    document.body.classList.add('text-open');
    textToggle.setAttribute('aria-expanded', 'true');
    try {
      await win.setIgnoreCursorEvents(false);
      await win.show();
      await invoke('set_conversation_window_size', { expanded: true });
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
      await invoke('set_conversation_window_size', { expanded: false });
      await win.setIgnoreCursorEvents(true);
    } catch (err) {
      console.error('Failed to close conversation panel:', err);
    }
  }
}

function selectPanel(name) {
  const showSetup = name === 'setup';
  chatView.hidden = showSetup;
  setupView.hidden = !showSetup;
  chatTab.classList.toggle('active', !showSetup);
  setupTab.classList.toggle('active', showSetup);
  chatTab.setAttribute('aria-selected', String(!showSetup));
  setupTab.setAttribute('aria-selected', String(showSetup));
  if (showSetup) {
    loadAppSettings();
    refreshHealth();
    refreshMemory();
  } else {
    textInput.focus();
  }
}

async function loadAppSettings() {
  settingsFeedback.textContent = '';
  try {
    const settings = await invoke('get_app_settings');
    ollamaUrlInput.value = settings.ollama_url;
    ollamaModelInput.value = settings.ollama_model;
    visionModelInput.value = settings.vision_model;
    whisperExeInput.value = settings.whisper_exe;
    whisperModelInput.value = settings.whisper_model;
    piperExeInput.value = settings.piper_exe;
    piperModelInput.value = settings.piper_model;
  } catch (err) {
    console.error('Failed to load AI settings:', err);
    settingsFeedback.textContent = 'Could not load settings';
  }
}

async function refreshHealth() {
  healthList.textContent = 'Checking components…';
  healthRefresh.disabled = true;
  try {
    const report = await invoke('get_system_health');
    healthList.replaceChildren();
    const components = [
      ['Microphone', report.microphone],
      ['Whisper', report.whisper_binary.available && report.whisper_model.available
        ? { available: true, details: report.whisper_binary.details }
        : { available: false, details: `${report.whisper_binary.details}; ${report.whisper_model.details}` }],
      ['Piper', report.piper_binary.available && report.piper_model.available
        ? { available: true, details: report.piper_binary.details }
        : { available: false, details: `${report.piper_binary.details}; ${report.piper_model.details}` }],
      ['Ollama', report.ollama_service],
      ['Memory', report.memory_store],
    ];
    for (const [name, component] of components) {
      const row = document.createElement('div');
      row.className = 'health-row';
      const label = document.createElement('span');
      label.textContent = name;
      const state = document.createElement('span');
      state.className = component.available ? 'available' : 'unavailable';
      state.textContent = component.available ? 'Ready' : 'Unavailable';
      state.title = component.details;
      row.append(label, state);
      healthList.appendChild(row);
    }
  } catch (err) {
    console.error('Health check failed:', err);
    healthList.textContent = 'Could not check system status';
  } finally {
    healthRefresh.disabled = false;
  }
}

async function refreshMemory() {
  memorySummary.textContent = 'Loading saved memory…';
  try {
    const memory = await invoke('get_memory_snapshot');
    const lines = [`Name: ${memory.user_profile.name || 'Not set'}`];
    const preferences = Object.entries(memory.user_profile.preferences || {});
    lines.push(preferences.length
      ? `Preferences: ${preferences.map(([key, value]) => `${key} = ${value}`).join(' · ')}`
      : 'Preferences: none');
    lines.push(memory.user_profile.habits?.length
      ? `Habits: ${memory.user_profile.habits.join(' · ')}`
      : 'Habits: none');
    lines.push(`Conversation history: ${memory.recent_conversation?.length || 0} messages`);
    lines.push(`Reminders: ${memory.scheduled_reminders?.length || 0}`);
    memorySummary.textContent = lines.join('\n');
  } catch (err) {
    console.error('Could not load saved memory:', err);
    memorySummary.textContent = 'Could not load saved memory';
  }
}

memoryRefresh.addEventListener('click', refreshMemory);

clearHistoryButton.addEventListener('click', async () => {
  if (!window.confirm('Clear Byte’s saved conversation history?')) return;
  clearHistoryButton.disabled = true;
  try {
    await invoke('clear_conversation_history');
    conversationMessages.replaceChildren();
    await refreshMemory();
  } catch (err) {
    memorySummary.textContent = `Could not clear history: ${err}`;
  } finally {
    clearHistoryButton.disabled = false;
  }
});

resetMemoryButton.addEventListener('click', async () => {
  if (!window.confirm('Reset Byte’s saved name, preferences, habits, reminders, and conversation history?')) return;
  resetMemoryButton.disabled = true;
  try {
    await invoke('reset_saved_memory');
    conversationMessages.replaceChildren();
    await refreshMemory();
  } catch (err) {
    memorySummary.textContent = `Could not reset saved memory: ${err}`;
  } finally {
    resetMemoryButton.disabled = false;
  }
});

chatTab.addEventListener('click', () => selectPanel('chat'));
setupTab.addEventListener('click', () => selectPanel('setup'));
healthRefresh.addEventListener('click', refreshHealth);

settingsForm.addEventListener('submit', async (event) => {
  event.preventDefault();
  settingsSave.disabled = true;
  settingsFeedback.textContent = 'Saving…';
  try {
    const settings = await invoke('save_app_settings', {
      ollamaUrl: ollamaUrlInput.value,
      ollamaModel: ollamaModelInput.value,
      visionModel: visionModelInput.value,
      whisperExe: whisperExeInput.value,
      whisperModel: whisperModelInput.value,
      piperExe: piperExeInput.value,
      piperModel: piperModelInput.value,
    });
    ollamaUrlInput.value = settings.ollama_url;
    ollamaModelInput.value = settings.ollama_model;
    visionModelInput.value = settings.vision_model;
    whisperExeInput.value = settings.whisper_exe;
    whisperModelInput.value = settings.whisper_model;
    piperExeInput.value = settings.piper_exe;
    piperModelInput.value = settings.piper_model;
    settingsFeedback.textContent = 'Saved';
    refreshHealth();
  } catch (err) {
    settingsFeedback.textContent = String(err);
  } finally {
    settingsSave.disabled = false;
  }
});

textToggle.addEventListener('click', () => setConversationOpen(true));
textClose.addEventListener('click', () => setConversationOpen(false));

textForm.addEventListener('submit', async (event) => {
  event.preventDefault();
  const text = textInput.value.trim();
  if (!text || isRunning) return;

  appendMessage('user', text);
  textInput.value = '';
  textSend.disabled = true;
  isRunning = true;
  try {
    const result = await invoke('start_text_interaction', { text });
    appendMessage('assistant', result.response);
  } catch (err) {
    console.error('Text interaction failed:', err);
    appendMessage('assistant', String(err));
  } finally {
    isRunning = false;
    textSend.disabled = false;
    textInput.focus();
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
    wakeToggle.textContent = 'Wake word: Error';
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
    wakeToggle.textContent = 'Wake word: Unavailable';
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
      appendMessage('user', result.transcription);
      appendMessage('assistant', result.response);
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
talkButton.addEventListener('click', async () => {
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

// Cursor ignore click-through outside orb bounds
presenceWidget.addEventListener('mouseenter', async () => {
  try {
    const { getCurrentWindow } = window.__TAURI__.window;
    await getCurrentWindow().setIgnoreCursorEvents(false);
  } catch (_) {}
});

presenceWidget.addEventListener('mouseleave', async () => {
  if (document.body.classList.contains('text-open')) return;
  try {
    const { getCurrentWindow } = window.__TAURI__.window;
    await getCurrentWindow().setIgnoreCursorEvents(true);
  } catch (_) {}
});

window.addEventListener('DOMContentLoaded', async () => {
  try {
    const { getCurrentWindow } = window.__TAURI__.window;
    await getCurrentWindow().setIgnoreCursorEvents(true);
    await invoke('reposition_presence');
  } catch (_) {}
});

// Initial state setup
updateStatusBadge('idle');
if (livingOrb) livingOrb.setState('idle');
