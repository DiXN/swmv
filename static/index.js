var app = new Vue({
  el: '#app',
  data: {
    loading: true,
    paths: null,
    mediaOverview: null,
    videoTypes: {
      mp4: 'video/mp4',
      webm: 'video/webm'
    }
  },
  mounted: async function () {
    const request = await fetch('http://127.0.0.1:8288/paths/')
    const paths = await request.json()
    this.paths = paths.map(p => `../../media/${p.replace(/\\/g, '/')}`)
    this.loading = false

    document.addEventListener('keydown', e => {
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

      const requestPath = `http://127.0.0.1:8288/file/exists/${path}`

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

        if (extension === 'mp4') {
          const filePath = `${path.slice(0, -4)}_thumbnail.mp4`
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
    }
  }
})
