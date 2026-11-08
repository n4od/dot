import { createApp } from 'vue'
import PetView from './pet/PetView.vue'
import SettingsView from './settings/SettingsView.vue'
import './styles.css'

/**
 * One bundle, several windows. Tauri loads the same index.html for every window
 * and tells us which one this is with a query parameter, so there is no
 * multi-page Vite config to keep in sync.
 */
const kind = new URLSearchParams(window.location.search).get('window') ?? 'pet'
document.documentElement.dataset.window = kind

createApp(kind === 'settings' ? SettingsView : PetView).mount('#app')