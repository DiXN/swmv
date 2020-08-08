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
        let index = +currentElement(this.mediaOverview) + 1

        if (index === this.paths.length)
          index = '0'
        else
          index = index.toString()

        this.mediaOverview = this.paths[index]
      }

      //previous media ->  arrow-key left || H
      if (this.mediaOverview && (e.code === 'ArrowLeft' || e.code === 'KeyH')) {
        let index = +currentElement(this.mediaOverview) - 1

        if (index < 0)
          index = (this.paths.length - 1).toString()
        else
          index = index.toString()

        this.mediaOverview = this.paths[index]
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
    }
  }
})
