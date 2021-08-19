var app = new Vue({
  el: '#app',
  data: {
    loading: true,
    reload: true,
    paths: null,
    thumbnails: null,
    mediaOverview: null,
    search: false,
    searchText: '',
    videoTypes: {
      mp4: 'video/mp4',
      webm: 'video/webm'
    }
  },
  mounted: async function () {

    const fileExistsOnServer = async (path) => {
      const requestPath = `thumbnails/${path}`

      try {
        const response = await fetch(requestPath, {
          method: 'HEAD',
          cache: 'no-cache'
        })

        return response.status === 200

      } catch(error) {
        return false
      }
    }

    const thumbnailHygiene = async (path) => {
      if (path) {
        const indexOfExtension = path.lastIndexOf('.') + 1
        const extension = path.substring(indexOfExtension, path.length)
        const fileName = path.substring(path.lastIndexOf('/'), path.length)

        if (extension === 'mp4') {
          const filePath = `thumbnails${fileName.slice(0, -4)}_thumbnail.mp4`
          const thumbnailPath = filePath.split('/').pop()

          const fileExists = await fileExistsOnServer(thumbnailPath)

          if (fileExists)
            return [true, filePath]
          else
            return [false, `${path}#t=2`]
        }

        if (extension === 'webm')
          return [true, `/media/${path}`]
      }

      return [false, path]
    }

    const checkThumbnails = async (paths) => {
      const filtered = paths.filter(p => p.endsWith('.mp4') || p.endsWith('webm'))

      const videos = document.querySelectorAll('video')

      for (const video of videos) {
        if (video.readyState === 0) {
          video.load()
        }
      }

      const thumbs = await Promise.all(filtered.map(async p => {
        const [auto, thumb] = await thumbnailHygiene(p)

        return [p, {
          'auto': auto,
          'thumbnail': thumb
        }]
      }))


      return new Map(thumbs) || null
    }

    (async function pathHandling() {
      let paths = null

      try {
        const request = await fetch('paths/')
        paths = await request.json()
      } catch(_) {
        if (this.app.reload)
          setTimeout(pathHandling, 2000)
      }

      const app = this.app
      app.thumbnails = await checkThumbnails(paths)
      app.paths = paths?.map(p => `../../media/${p.replace(/\\/g, '/')}`)
      app.loading = false

      if (app.reload)
        setTimeout(pathHandling, 2000)
    })()

    document.addEventListener('keydown', e => {
      //search
      if (e.keyCode === 114 || (e.ctrlKey && e.keyCode === 70)) {
        e.preventDefault()

        if (this.search) {
          this.search = false
        } else {
          this.search = true

          setTimeout(() => {
            document.getElementById('search').focus()
          }, 200);
        }
      }

      const currentElement = fileName => this.paths.findIndex(e => e === fileName)

      //next media ->  arrow-key right || L
      if (this.mediaOverview && (e.code === 'ArrowRight' || e.code === 'KeyL')) {
        const video = document.querySelector('.media-overview video')
        video && video.pause()

        let index = +currentElement(this.mediaOverview) + 1

        if (index === this.paths.length)
          index = '0'
        else
          index = index.toString()

        this.mediaOverview = this.paths[index]
      }

      //previous media ->  arrow-key left || H
      if (this.mediaOverview && (e.code === 'ArrowLeft' || e.code === 'KeyH')) {
        const video = document.querySelector('.media-overview video')
        video && video.pause()

        let index = +currentElement(this.mediaOverview) - 1

        if (index < 0)
          index = (this.paths.length - 1).toString()
        else
          index = index.toString()

        this.mediaOverview = this.paths[index]
      }

      //close overview ->  escape || backspace
      if (this.mediaOverview && (e.code === 'Escape' || e.code === 'Backspace')) {
        this.closeMedia()
      }

      if (this.mediaOverview && e.code === 'KeyF') {
        this.zoom()
      }

    })
  },
  methods: {
    getExtension: function(path) {
      if (path) {
        const indexOfExtension = path.lastIndexOf('.') + 1
        const extension = path.substring(indexOfExtension, path.length)
        return this.videoTypes[extension]
      }

      return this.videoTypes.mp4
    },
    thumbnailHygiene: function(path) {
      const actual = path.substring('../../media/'.length, path.length)

      const thumb = this.thumbnails.get(actual)

      if (thumb) {
        const { auto, thumbnail } = thumb
        return thumbnail
      }

      return path
    },
    isAutoplay: function(path) {
      const actual = path.substring('../../media/'.length, path.length)

      const thumb = this.thumbnails.get(actual)

      if (thumb) {
        const { auto, thumbnail } = thumb
        return auto
      }

      return false
    },
    getFileName: function(path) {
      if (path) {
        const indexOfSlash = path.lastIndexOf('/') + 1
        return path.substring(indexOfSlash, path.length)
      }
    },
    zoom: function() {
      const img = document.querySelector('.media-overview img, .media-overview video')

      if (img.style.width === '100%') {
        img.style.width = '80%'
        img.style.height = '80%'
      } else {
        img.style.width = '100%'
        img.style.height = '100%'
      }
    },
    closeMedia: function() {
      const video = document.querySelector('.media-overview video')
      video && video.pause()

      this.mediaOverview = null
    }
  }
})
