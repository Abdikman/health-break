const { invoke, convertFileSrc } = window.__TAURI__.core

// Функция настройки при старте
export async function setupAudio() {
	const select = document.getElementById('sound-select')
	const openBtn = document.getElementById('open-sounds-btn')

	if (!select) return

	try {
		const sounds = await invoke('get_sounds')

		select.innerHTML = '<option value="none">Без звука</option>'

		// 1. Читаем сохраненный путь при загрузке
		const savedPath = localStorage.getItem('hb-sound-path')

		sounds.forEach(s => {
			const opt = document.createElement('option')
			opt.textContent = s.name
			// Если файл кастомный, конвертируем путь для asset protocol
			opt.value = s.is_custom ? convertFileSrc(s.path) : s.path
			select.appendChild(opt)
		})

		// 2. Если есть сохраненный путь, выбираем его в списке
		if (savedPath) {
			select.value = savedPath
		}

		// 3. Слушаем изменения
		select.addEventListener('change', () => {
			const newValue = select.value
			localStorage.setItem('hb-sound-path', newValue)

			// Сразу тестируем звук, передавая путь
			playSoundWithUrl(newValue)
		})

		if (openBtn) {
			openBtn.addEventListener('click', () => invoke('open_sounds_folder'))
		}
	} catch (e) {
		console.error('Audio init error:', e)
	}
}

// Главная функция воспроизведения (экспортируемая)
export function playSound() {
	// 1. Читаем актуальное значение из хранилища
	const pathFromStorage = localStorage.getItem('hb-sound-path')

	// 2. Если пути нет или он 'none', ничего не делаем
	if (!pathFromStorage || pathFromStorage === 'none') return

	// 3. Запускаем
	playSoundWithUrl(pathFromStorage)
}

// Вспомогательная функция (внутренняя)
function playSoundWithUrl(url) {
	if (!url || url === 'none') return

	console.log('Playing sound:', url)
	new Audio(url).play().catch(e => {
		console.error('Playback failed:', e)
	})
}
