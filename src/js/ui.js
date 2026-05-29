const { invoke } = window.__TAURI__.core
const { listen } = window.__TAURI__.event

// 1. Настройка переключения вкладок
export function setupTabs() {
	const buttons = document.querySelectorAll('.nav-btn')
	const tabs = document.querySelectorAll('.tab-content')

	// Слушаем событие "перейти в каталог" от Rust (например, при клике на уведомление)
	listen('navigate-to-catalog', () => {
		const btn = document.querySelector('[data-tab="catalog"]')
		if (btn) btn.click()
	})

	buttons.forEach(btn => {
		btn.addEventListener('click', () => {
			// Убираем активность со всех кнопок и вкладок
			buttons.forEach(b => b.classList.remove('active'))
			tabs.forEach(t => t.classList.remove('active'))

			// Активируем текущую
			btn.classList.add('active')
			const tabId = btn.dataset.tab // например, "home", "catalog"
			const tab = document.getElementById(tabId)
			if (tab) tab.classList.add('active')
		})
	})
}

// 2. Настройка каталога
export function setupCatalog() {
	const btn = document.querySelector('[data-tab="catalog"]')
	const grid = document.getElementById('exercise-grid')
	const modal = document.getElementById('exercise-modal')
	const closeBtn = document.querySelector('.close-modal')

	// Элементы модального окна
	const mTitle = document.getElementById('modal-title')
	const mDesc = document.getElementById('modal-desc')
	const mMedia = document.getElementById('modal-media')

	// Закрытие модального окна
	if (closeBtn) closeBtn.onclick = () => modal.classList.remove('active')
	window.onclick = e => {
		if (e.target === modal) modal.classList.remove('active')
	}

	// Загрузка данных при клике на вкладку "Каталог"
	if (btn) {
		btn.addEventListener('click', async () => {
			try {
				const exercises = await invoke('get_exercises')

				if (grid) {
					grid.innerHTML = exercises
						.map(
							ex => `
                        <div class="exercise-card" data-id="${ex.id}">
                            <div class="card-img-container">
                                <img src="${ex.image_url}" class="card-img" alt="${ex.title}">
                            </div>
                            <div class="card-info">
                                <div class="card-title">${ex.title}</div>
                            </div>
                        </div>
                    `,
						)
						.join('')

					document.querySelectorAll('.exercise-card').forEach(card => {
						card.addEventListener('click', () => {
							const id = parseInt(card.dataset.id)
							const data = exercises.find(e => e.id === id)
							if (data) openModal(data)
						})
					})
				}
			} catch (e) {
				console.error('Ошибка загрузки каталога:', e)
			}
		})
	}

	function openModal(data) {
		if (!modal) return
		mTitle.textContent = data.title
		mDesc.textContent = data.description
		// Если image_url пустой, можно поставить заглушку
		mMedia.innerHTML = `<img src="${data.image_url}" alt="${data.title}">`
		modal.classList.add('active')
	}
}

export function setupSettings() {
	const durationInput = document.getElementById('duration-input')
	if (durationInput) {
		durationInput.addEventListener('change', () => {
			const val = parseInt(durationInput.value)
			if (val > 0) {
				invoke('set_duration', { minutes: val })
					.then(() => console.log('Время изменено на', val))
					.catch(console.error)
			}
		})
	}
}
