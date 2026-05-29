import { playSound } from './audio.js'

const { invoke } = window.__TAURI__.core
const { listen } = window.__TAURI__.event

let isRunning = false
let ignoreTicks = false

export async function setupTimer() {
	const toggleBtn = document.getElementById('toggle-timer')
	const display = document.querySelector('.timer-display')

	// Получаем начальное время из настроек, чтобы при стопе возвращать его
	const defaultTime = '30:00'

	if (toggleBtn) {
		toggleBtn.addEventListener('click', () => {
			if (!isRunning) {
				// --- СТАРТ ---
				console.log('User clicked Start')
				ignoreTicks = false
				invoke('start_timer')

				toggleBtn.classList.add('active')
				isRunning = true
			} else {
				// --- СТОП ---
				console.log('User clicked Stop')
				ignoreTicks = true

				invoke('stop_timer')

				// Мгновенно выключаем интерфейс
				toggleBtn.classList.remove('active')
				isRunning = false
			}
		})
	}

	// Слушаем тик таймера
	await listen('timer-tick', event => {
		// Если мы нажали стоп, мы игнорируем всё, что долетает от Rust
		if (ignoreTicks) return

		const sec = event.payload
		const m = Math.floor(sec / 60)
			.toString()
			.padStart(2, '0')
		const s = (sec % 60).toString().padStart(2, '0')

		if (display) display.textContent = `${m}:${s}`
	})

	// Слушаем завершение
	await listen('timer-finished', () => {
		// Если это реальное завершение, а не принудительный стоп
		if (ignoreTicks) return

		if (display) display.textContent = '00:00'
		if (toggleBtn) toggleBtn.classList.remove('active')
		isRunning = false

		playSound()
		invoke('show_notification', {
			title: 'HealthBreak',
			body: 'Время вышло! Пора отдохнуть.',
		}).catch(console.error)
	})
}
