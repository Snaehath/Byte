const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;

const orb = document.getElementById("byte-orb");
const statusText = document.getElementById("status-text");
const assistantWidget = document.querySelector(".assistant-widget");

let hideTimeout = null;
let isRecording = false;

// Function to run the full conversational lifecycle
async function runInteraction() {
  if (isRecording) return;
  isRecording = true;

  if (hideTimeout) {
    clearTimeout(hideTimeout);
    hideTimeout = null;
  }

  // Ensure window is visible and focused
  try {
    const { getCurrentWindow } = window.__TAURI__.window;
    const win = getCurrentWindow();
    await win.show();
    await win.setFocus();
    assistantWidget.classList.add("visible");
  } catch (err) {
    console.error("Failed to show/focus window:", err);
  }

  orb.className = "orb state-listening";
  statusText.innerText = "Listening...";

  try {
    const result = await invoke("start_interaction");
    console.log("Interaction completed:", result);
    if (result && result.auto_listen) {
      setTimeout(() => {
        isRecording = false;
        runInteraction();
      }, 300);
      return;
    }
  } catch (err) {
    console.error("Interaction error:", err);
  } finally {
    resetToIdle();
  }
}

orb.addEventListener("click", async () => {
  if (!isRecording) {
    runInteraction();
  } else {
    // Force stop recording early if user clicks the orb during capture
    try {
      await invoke("stop_action");
    } catch (err) {
      console.error("Error stopping recording:", err);
    }
  }
});

// Tauri Event Subscriptions from Rust
listen("processing", () => {
  orb.className = "orb state-processing";
  statusText.innerText = "Thinking...";
});

listen("speaking", (event) => {
  const mood = event.payload || "calm";
  orb.className = `orb state-speaking mood-${mood}`;
  statusText.innerText = "Speaking...";
});

listen("wakeup", () => {
  console.log("Wake word triggered!");
  runInteraction();
});

// Local keyboard shortcut fallback: Ctrl+B to start/open the assistant
window.addEventListener("keydown", (event) => {
  if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "b") {
    event.preventDefault();
    if (!isRecording) {
      runInteraction();
    }
  }
});

// Auto-hide on blur (clicking outside) with debounce
let blurTimeout = null;
window.addEventListener("blur", () => {
  blurTimeout = setTimeout(async () => {
    try {
      if (isRecording) {
        await invoke("stop_action");
      }
    } catch (err) {
      console.error("Failed to stop recording on blur:", err);
    }
  }, 150);
});

window.addEventListener("focus", () => {
  if (blurTimeout) {
    clearTimeout(blurTimeout);
    blurTimeout = null;
  }
});

function resetToIdle() {
  isRecording = false;
  orb.className = "orb state-idle";
  statusText.innerText = "Click or say 'Hey Byte'";

  if (hideTimeout) {
    clearTimeout(hideTimeout);
  }
}

// Cursor ignore click-through settings
const orbContainer = document.querySelector(".orb-container");

orbContainer.addEventListener("mouseenter", async () => {
  try {
    const { getCurrentWindow } = window.__TAURI__.window;
    await getCurrentWindow().setIgnoreCursorEvents(false);
  } catch (err) {
    console.error("Failed to disable ignore cursor events:", err);
  }
});

orbContainer.addEventListener("mouseleave", async () => {
  try {
    const { getCurrentWindow } = window.__TAURI__.window;
    await getCurrentWindow().setIgnoreCursorEvents(true);
  } catch (err) {
    console.error("Failed to enable ignore cursor events:", err);
  }
});

// Set initial window state to ignore cursor events when loading
window.addEventListener("DOMContentLoaded", async () => {
  try {
    const { getCurrentWindow } = window.__TAURI__.window;
    await getCurrentWindow().setIgnoreCursorEvents(true);
    assistantWidget.classList.add("visible");
  } catch (err) {
    console.error("Failed to set initial ignore cursor events state:", err);
  }
});

// Set initial state
resetToIdle();
