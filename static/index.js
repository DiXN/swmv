var app = new Vue({
  el: '#app',
  data: {
    loading: true,
    reload: true,
    paths: null,
    mediaOverview: null,
    search: false,
    searchText: '',
    videoTypes: {
      mp4: 'video/mp4',
      webm: 'video/webm'
    }
  },
  mounted: async function () {
    (async function pathHandling() {
      const request = await fetch('http://0.0.0.0:8288/paths/')
      const paths = await request.json()

      const app = this.app
      app.paths = paths.map(p => `../../media/${p.replace(/\\/g, '/')}`)
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
        const video = document.querySelector('.media-overview video')
        video && video.pause()

        this.mediaOverview = null
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
    fileExistsOnServer: function(path) {
      const http = new XMLHttpRequest()

      const requestPath = `http://0.0.0.0:8288/file/exists/${path}`

      http.open('GET', requestPath, false)
      http.send()

      if (http.status === 200)
        if (JSON.parse(http.responseText))
          return true

      return false
    },
    thumbnailHygiene: function(path) {
      if (path) {
        const indexOfExtension = path.lastIndexOf('.') + 1
        const extension = path.substring(indexOfExtension, path.length)
        const fileName = path.substring(path.lastIndexOf('/'), path.length)

        if (extension === 'mp4') {
          const filePath = `thumbnails${fileName.slice(0, -4)}_thumbnail.mp4`
          const thumbnailPath = filePath.split('/').pop()

          const fileExists = this.fileExistsOnServer(thumbnailPath)
          if (fileExists)
            return filePath
          else
            return `${path}#t=2`
        }
      }

      return path
    },
    isAutoplay: function(path) {
      if (path) {
        const indexOfExtension = path.lastIndexOf('.') + 1
        const extension = path.substring(indexOfExtension, path.length)

        if (extension === 'mp4') {
          const thumbnailPath = `${path.split('/').pop().slice(0, -4)}_thumbnail.mp4`
          const fileExists = this.fileExistsOnServer(thumbnailPath)
          if (fileExists)
            return true
          else
            return false
        }
      }

      return true
    },
    getFileName: function(path) {
      if (path) {
        const indexOfSlash = path.lastIndexOf('/') + 1
        return path.substring(indexOfSlash, path.length)
      }
    }
  }
})
