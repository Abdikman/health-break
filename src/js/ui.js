const { invoke, convertFileSrc } = window.__TAURI__.core
const { listen } = window.__TAURI__.event

// 1. Настройка переключения вкладок
export function setupTabs() {
	const buttons = document.querySelectorAll('.nav-btn')
	const tabs = document.querySelectorAll('.tab-content')

	listen('navigate-to-catalog', () => {
		const btn = document.querySelector('[data-tab="catalog"]')
		if (btn) btn.click()
	})

	buttons.forEach(btn => {
		btn.addEventListener('click', () => {
			buttons.forEach(b => b.classList.remove('active'))
			tabs.forEach(t => t.classList.remove('active'))
			btn.classList.add('active')
			const tab = document.getElementById(btn.dataset.tab)
			if (tab) tab.classList.add('active')
		})
	})
}

// 2. Настройка каталога
export function setupCatalog() {
	const catalogTabBtn = document.querySelector('[data-tab="catalog"]')
	const grid = document.getElementById('exercise-grid')

	// --- Модалка просмотра ---
	const modal = document.getElementById('exercise-modal')
	const closeBtn = modal.querySelector('.close-modal')
	const mTitle = document.getElementById('modal-title')
	const mDesc = document.getElementById('modal-desc')
	const mMedia = document.getElementById('modal-media')
	const mVideoContainer = document.getElementById('modal-video-container')
	const mVideo = document.getElementById('modal-video')
	const mAudioContainer = document.getElementById('modal-audio-container')
	const mAudio = document.getElementById('modal-audio')
	const mTags = document.getElementById('modal-tags')
	const deleteBtn = document.getElementById('delete-exercise-btn')

	// --- Модалка добавления ---
	const addBtn = document.getElementById('add-exercise-btn')
	const addModal = document.getElementById('add-exercise-modal')
	const closeAddBtn = document.getElementById('close-add-modal')
	const addForm = document.getElementById('add-exercise-form')
	const pickImagesBtn = document.getElementById('pick-images-btn')
	const imagePreviewStrip = document.getElementById('image-preview-strip')
	const pickAudioBtn = document.getElementById('pick-audio-btn')
	const audioFileLabel = document.getElementById('audio-file-label')
	const pickVideoBtn = document.getElementById('pick-video-btn')
	const videoFileLabel = document.getElementById('video-file-label')
	const cancelAddBtn = document.getElementById('cancel-add-exercise')

	// --- Состояние ---
	let exercises = []
	let currentExercise = null
	// Форма
	let selectedImagePaths = []
	let selectedAudioPath = null
	let selectedVideoPath = null
	// Галерея в модалке просмотра
	let galleryImages = []
	let currentImageIndex = 0
	let galleryMainImg = null
	let galleryThumbStrip = null

	// ---- Вспомогательные функции ----

	// Нормализует картинки упражнения в массив src.
	// builtin: image_url — статический asset, convertFileSrc не нужен.
	// user: image_paths — произвольные пути, нужен convertFileSrc.
	function getImages(ex) {
		if (ex.image_paths && ex.image_paths.length > 0) {
			return ex.image_paths.map(p => convertFileSrc(p))
		}
		if (ex.image_url) return [ex.image_url]
		return []
	}

	// Переключает галерею на указанный индекс (или сдвигает на delta от текущего).
	// Зацикливание в обе стороны.
	function navigateGallery(delta) {
		if (galleryImages.length <= 1) return
		currentImageIndex = (currentImageIndex + delta + galleryImages.length) % galleryImages.length
		if (galleryMainImg) galleryMainImg.src = galleryImages[currentImageIndex]
		if (galleryThumbStrip) {
			galleryThumbStrip.querySelectorAll('.modal-gallery-thumb').forEach((t, i) => {
				t.classList.toggle('active', i === currentImageIndex)
			})
		}
	}

	// Обработчик клавиатурной навигации: добавляется при открытии модалки,
	// снимается при закрытии. Не перехватывает стрелки, если фокус на видео-плеере.
	function handleGalleryKeys(e) {
		if (e.key !== 'ArrowLeft' && e.key !== 'ArrowRight') return
		if (galleryImages.length <= 1) return
		if (mVideo && document.activeElement === mVideo) return
		e.preventDefault()
		navigateGallery(e.key === 'ArrowRight' ? 1 : -1)
	}

	// ---- Модалка просмотра ----

	closeBtn.onclick = () => closeDetailModal()
	window.addEventListener('click', e => {
		if (e.target === modal) closeDetailModal()
		if (e.target === addModal) closeAddModal()
	})

	function closeDetailModal() {
		document.removeEventListener('keydown', handleGalleryKeys)
		modal.classList.remove('active')
		if (mVideo) { mVideo.pause(); mVideo.src = '' }
		if (mAudio) { mAudio.pause(); mAudio.src = '' }
		if (mMedia) mMedia.innerHTML = ''
		galleryImages = []
		currentImageIndex = 0
		galleryMainImg = null
		galleryThumbStrip = null
		currentExercise = null
	}

	deleteBtn?.addEventListener('click', async () => {
		if (!currentExercise || currentExercise.source !== 'user') return
		try {
			await invoke('delete_user_exercise', { id: currentExercise.id })
			closeDetailModal()
			await loadExercises()
		} catch (e) {
			console.error('Ошибка удаления упражнения:', e)
		}
	})

	async function openDetailModal(data) {
		currentExercise = data
		mTitle.textContent = data.title
		mDesc.textContent = data.description

		if (mTags) {
			mTags.innerHTML = data.tags && data.tags.length
				? data.tags.map(t => `<span class="tag">${t}</span>`).join('')
				: ''
		}

		// Видео — приоритетнее картинок
		if (mVideo && mVideoContainer) {
			if (data.video_path) {
				mVideo.src = convertFileSrc(data.video_path)
				mVideoContainer.style.display = 'block'
			} else {
				mVideoContainer.style.display = 'none'
				mVideo.src = ''
			}
		}

		// Галерея картинок
		renderModalGallery(getImages(data))

		// Аудио: base64 через Rust, чтобы не расширять assetProtocol.scope
		if (mAudio && mAudioContainer) {
			if (data.audio_path) {
				try {
					const b64 = await invoke('get_exercise_audio_data', { path: data.audio_path })
					const ext = data.audio_path.split('.').pop().toLowerCase()
					const mime = ext === 'mp3' ? 'audio/mpeg' : 'audio/wav'
					mAudio.src = `data:${mime};base64,${b64}`
					mAudioContainer.style.display = 'block'
				} catch (e) {
					console.error('Ошибка загрузки аудио:', e)
					mAudioContainer.style.display = 'none'
				}
			} else {
				mAudioContainer.style.display = 'none'
				mAudio.src = ''
			}
		}

		if (deleteBtn) {
			deleteBtn.style.display = data.source === 'user' ? 'inline-flex' : 'none'
		}

		// Подключаем клавиатурную навигацию — removeEventListener перед add
		// гарантирует отсутствие дублей при повторном открытии без явного закрытия
		document.removeEventListener('keydown', handleGalleryKeys)
		document.addEventListener('keydown', handleGalleryKeys)

		modal.classList.add('active')
	}

	// Рендерит блок картинок в mMedia.
	// 0 фото → пусто. 1 фото → одиночный <img>. 2+ → крупный слайд + стрелки + полоска миниатюр.
	function renderModalGallery(images) {
		mMedia.innerHTML = ''
		galleryImages = images
		currentImageIndex = 0
		galleryMainImg = null
		galleryThumbStrip = null

		if (images.length === 0) return

		if (images.length === 1) {
			const img = document.createElement('img')
			img.src = images[0]
			img.className = 'modal-img'
			mMedia.appendChild(img)
			return
		}

		// Галерея: обёртка с position:relative для стрелок
		const gallery = document.createElement('div')
		gallery.className = 'modal-gallery'

		const wrap = document.createElement('div')
		wrap.className = 'modal-gallery-main-wrap'

		const mainImg = document.createElement('img')
		mainImg.className = 'modal-gallery-main'
		mainImg.src = images[0]
		galleryMainImg = mainImg

		const prevBtn = document.createElement('button')
		prevBtn.type = 'button'
		prevBtn.className = 'gallery-arrow gallery-arrow--prev'
		prevBtn.innerHTML = '&#8249;'
		prevBtn.setAttribute('aria-label', 'Предыдущая картинка')
		prevBtn.addEventListener('click', () => navigateGallery(-1))

		const nextBtn = document.createElement('button')
		nextBtn.type = 'button'
		nextBtn.className = 'gallery-arrow gallery-arrow--next'
		nextBtn.innerHTML = '&#8250;'
		nextBtn.setAttribute('aria-label', 'Следующая картинка')
		nextBtn.addEventListener('click', () => navigateGallery(1))

		wrap.appendChild(mainImg)
		wrap.appendChild(prevBtn)
		wrap.appendChild(nextBtn)
		gallery.appendChild(wrap)

		const strip = document.createElement('div')
		strip.className = 'modal-gallery-strip'
		galleryThumbStrip = strip

		images.forEach((src, i) => {
			const thumb = document.createElement('img')
			thumb.src = src
			thumb.className = 'modal-gallery-thumb' + (i === 0 ? ' active' : '')
			thumb.addEventListener('click', () => {
				currentImageIndex = i
				navigateGallery(0)
			})
			strip.appendChild(thumb)
		})
		gallery.appendChild(strip)

		mMedia.appendChild(gallery)
	}

	// ---- Модалка добавления ----

	addBtn?.addEventListener('click', () => {
		addModal.classList.add('active')
	})

	closeAddBtn?.addEventListener('click', () => closeAddModal())
	cancelAddBtn?.addEventListener('click', () => closeAddModal())

	function closeAddModal() {
		addModal.classList.remove('active')
		addForm.reset()
		selectedImagePaths = []
		selectedAudioPath = null
		selectedVideoPath = null
		if (imagePreviewStrip) imagePreviewStrip.innerHTML = ''
		if (audioFileLabel) audioFileLabel.textContent = 'Не выбрано'
		if (videoFileLabel) videoFileLabel.textContent = 'Не выбрано'
	}

	function renderImagePreviews() {
		if (!imagePreviewStrip) return
		imagePreviewStrip.innerHTML = ''
		selectedImagePaths.forEach((path, i) => {
			const item = document.createElement('div')
			item.className = 'image-preview-item'

			const img = document.createElement('img')
			img.src = convertFileSrc(path)

			const removeBtn = document.createElement('button')
			removeBtn.type = 'button'
			removeBtn.className = 'remove-img-btn'
			removeBtn.textContent = '×'
			removeBtn.addEventListener('click', () => {
				selectedImagePaths.splice(i, 1)
				renderImagePreviews()
			})

			item.appendChild(img)
			item.appendChild(removeBtn)
			imagePreviewStrip.appendChild(item)
		})
	}

	pickImagesBtn?.addEventListener('click', async () => {
		try {
			const paths = await invoke('pick_exercise_images')
			if (paths && paths.length > 0) {
				selectedImagePaths.push(...paths)
				renderImagePreviews()
			}
		} catch (e) {
			console.error('Ошибка выбора изображений:', e)
		}
	})

	pickAudioBtn?.addEventListener('click', async () => {
		try {
			const path = await invoke('pick_exercise_audio')
			if (path) {
				selectedAudioPath = path
				const fileName = path.split('\\').pop().split('/').pop()
				if (audioFileLabel) audioFileLabel.textContent = fileName
			}
		} catch (e) {
			console.error('Ошибка выбора аудио:', e)
		}
	})

	pickVideoBtn?.addEventListener('click', async () => {
		try {
			const path = await invoke('pick_exercise_video')
			if (path) {
				selectedVideoPath = path
				const fileName = path.split('\\').pop().split('/').pop()
				if (videoFileLabel) videoFileLabel.textContent = fileName
			}
		} catch (e) {
			console.error('Ошибка выбора видео:', e)
		}
	})

	addForm?.addEventListener('submit', async e => {
		e.preventDefault()

		const title = document.getElementById('new-ex-title').value.trim()
		const description = document.getElementById('new-ex-desc').value.trim()
		const checked = addForm.querySelectorAll('input[name="tag"]:checked')
		const tags = Array.from(checked).map(cb => cb.value)

		if (!title || !description) return

		try {
			await invoke('add_user_exercise', {
				payload: {
					title,
					description,
					tags,
					image_paths: selectedImagePaths,
					audio_path: selectedAudioPath,
					video_path: selectedVideoPath,
				},
			})
			closeAddModal()
			await loadExercises()
		} catch (e) {
			console.error('Ошибка добавления упражнения:', e)
		}
	})

	// ---- Сетка упражнений ----

	async function loadExercises() {
		try {
			exercises = await invoke('get_exercises')
			renderGrid()
		} catch (e) {
			console.error('Ошибка загрузки каталога:', e)
		}
	}

	function renderGrid() {
		if (!grid) return

		grid.innerHTML = exercises
			.map(ex => {
				const isUser = ex.source === 'user'
				const images = getImages(ex)

				const imgContent = images.length > 0
					? `<img src="${images[0]}" class="card-img" alt="${ex.title}">`
					: '<div class="card-img-placeholder">🏃</div>'

				const tagsHtml = ex.tags && ex.tags.length
					? `<div class="card-tags">${ex.tags.map(t => `<span class="tag">${t}</span>`).join('')}</div>`
					: ''

				const mediaBadge = ex.video_path
					? '<span class="video-badge">🎬</span>'
					: ex.audio_path ? '<span class="audio-badge">🎵</span>' : ''

				const imgCountBadge = images.length > 1
					? `<span class="img-count-badge">🖼 ${images.length}</span>`
					: ''

				return `
					<div class="exercise-card${isUser ? ' user-exercise' : ''}" data-id="${ex.id}">
						<div class="card-img-container">
							${imgContent}
							${isUser ? '<span class="user-badge">Моё</span>' : ''}
							${mediaBadge}
							${imgCountBadge}
						</div>
						<div class="card-info">
							<div class="card-title">${ex.title}</div>
							${tagsHtml}
						</div>
					</div>
				`
			})
			.join('')

		grid.querySelectorAll('.exercise-card').forEach(card => {
			card.addEventListener('click', () => {
				const id = card.dataset.id
				const data = exercises.find(e => e.id === id)
				if (data) openDetailModal(data)
			})
		})
	}

	if (catalogTabBtn) {
		catalogTabBtn.addEventListener('click', loadExercises)
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
