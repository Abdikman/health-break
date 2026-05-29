import { setupAudio } from './audio.js'
import { setupTimer } from './timer.js'
import { setupTabs, setupCatalog, setupSettings } from './ui.js'

import { setupDebug } from './debug.js'

window.addEventListener('DOMContentLoaded', () => {
	console.log('App Initializing...')

	setupTabs()
	setupCatalog()
	setupSettings()
	setupTimer()
	setupAudio()

	setupDebug()
})
