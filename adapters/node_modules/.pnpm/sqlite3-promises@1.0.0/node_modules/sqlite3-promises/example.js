const sqlite3 = require('sqlite3').verbose()

var db

async function open () {
  const dbh = await new sqlite3.Database(':memory:', error => {
    if (error) throw error
  })

  db = require('.')(dbh)
}

async function example () {
  await db.exec('CREATE TABLE foo(id INTEGER PRIMARY KEY AUTOINCREMENT, x, y)')
  console.log(await db.run('INSERT INTO foo(x,y) VALUES(2,3)'))
  console.log(await db.run('INSERT INTO foo(x,y) VALUES(4,5)'))
  console.log(await db.all('SELECT * FROM foo'))
  console.log(await db.get('SELECT * FROM foo WHERE id=1'))
}

async function main () {
  await open()
  await example()
}

main()
