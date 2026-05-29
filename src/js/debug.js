import { playSound } from './audio.js' // Импортируем звук, чтобы проверить и его тоже

const { invoke } = window.__TAURI__.core

export function setupDebug() {
	const testBtn = document.getElementById('test-notify-btn')

	if (testBtn) {
		testBtn.addEventListener('click', () => {
			console.log('Test button clicked')

			// 1. Тестируем звук
			playSound()

			// 2. Тестируем уведомление
			invoke('show_notification', {
				title: 'Тестовое уведомление',
				body: 'Если вы это видите и слышите звук — всё работает!',
			}).catch(console.error)
		})
	}
}
