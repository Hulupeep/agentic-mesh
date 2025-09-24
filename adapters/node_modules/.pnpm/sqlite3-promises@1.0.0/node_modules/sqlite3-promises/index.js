const sqlite3 = require('sqlite3')

module.exports = function (db) {
  return {
    exec: function (...args) {
      return new Promise(function (resolve, reject) {
        db.exec(...args, function (error) {
          if (error) reject(error)
          else resolve()
        })
      })
    },

    run: function (...args) {
      return new Promise(function (resolve, reject) {
        db.run(...args, function (error) {
          if (error) reject(error)
          else resolve({ changes: this.changes, lastID: this.lastID })
        })
      })
    },

    get: function (...args) {
      return new Promise(function (resolve, reject) {
        db.get(...args, function (error, row) {
          if (error) reject(error)
          else resolve(row)
        })
      })
    },

    all: function (...args) {
      return new Promise(function (resolve, reject) {
        db.all(...args, function (error, rows) {
          if (error) reject(error)
          else resolve(rows)
        })
      })
    },
  }
}
