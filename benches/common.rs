#![feature(generic_associated_types)]
use redb::ReadableTable;
use redb::TableDefinition;
use std::fs;
use std::fs::File;
use std::path::Path;

#[allow(dead_code)]
const X: TableDefinition<[u8], [u8]> = TableDefinition::new("x");

pub trait BenchDatabase {
    type W<'db>: BenchWriteTransaction
    where
        Self: 'db;
    type R<'db>: BenchReadTransaction
    where
        Self: 'db;

    fn db_type_name() -> &'static str;

    fn write_transaction(&mut self) -> Self::W<'_>;

    fn read_transaction(&self) -> Self::R<'_>;
}

pub trait BenchWriteTransaction {
    type W<'txn>: BenchInserter
    where
        Self: 'txn;

    fn get_inserter(&mut self) -> Self::W<'_>;

    #[allow(clippy::result_unit_err)]
    fn commit(self) -> Result<(), ()>;
}

pub trait BenchInserter {
    #[allow(clippy::result_unit_err)]
    fn insert(&mut self, key: &[u8], value: &[u8]) -> Result<(), ()>;

    #[allow(clippy::result_unit_err)]
    fn remove(&mut self, key: &[u8]) -> Result<(), ()>;
}

pub trait BenchReadTransaction {
    type T<'txn>: BenchReader
    where
        Self: 'txn;

    fn get_reader(&self) -> Self::T<'_>;
}

pub trait BenchReader {
    type Output<'a>: AsRef<[u8]> + 'a
    where
        Self: 'a;
    type Iterator<'a>: BenchIterator
    where
        Self: 'a;

    fn get<'a>(&'a self, key: &[u8]) -> Option<Self::Output<'a>>;

    fn range_from<'a>(&'a self, start: &'a [u8]) -> Self::Iterator<'a>;
}

pub trait BenchIterator {
    type Output<'a>: AsRef<[u8]> + 'a
    where
        Self: 'a;

    fn next(&mut self) -> Option<(Self::Output<'_>, Self::Output<'_>)>;
}

pub struct RedbBenchDatabase<'db> {
    db: &'db redb::Database,
}

impl<'db> RedbBenchDatabase<'db> {
    #[allow(dead_code)]
    pub fn new(db: &'db redb::Database) -> Self {
        RedbBenchDatabase { db }
    }
}

impl<'db> BenchDatabase for RedbBenchDatabase<'db> {
    type W<'a> = RedbBenchWriteTransaction<'a>
    where
        Self: 'a;
    type R<'a> = RedbBenchReadTransaction<'a>
    where
        Self: 'a;

    fn db_type_name() -> &'static str {
        "redb"
    }

    fn write_transaction(&mut self) -> Self::W<'_> {
        let txn = self.db.begin_write().unwrap();
        RedbBenchWriteTransaction { txn }
    }

    fn read_transaction(&self) -> Self::R<'_> {
        let txn = self.db.begin_read().unwrap();
        RedbBenchReadTransaction { txn }
    }
}

pub struct RedbBenchReadTransaction<'a> {
    txn: redb::ReadTransaction<'a>,
}

impl<'db> BenchReadTransaction for RedbBenchReadTransaction<'db> {
    type T<'txn> = RedbBenchReader<'txn>
    where
        Self: 'txn;

    fn get_reader(&self) -> Self::T<'_> {
        let table = self.txn.open_table(X).unwrap();
        RedbBenchReader { table }
    }
}

pub struct RedbBenchReader<'txn> {
    table: redb::ReadOnlyTable<'txn, [u8], [u8]>,
}

impl<'txn> BenchReader for RedbBenchReader<'txn> {
    type Output<'a> = &'a [u8]
    where
        Self: 'a;
    type Iterator<'a> = RedbBenchIterator<'a>
    where
        Self: 'a;

    fn get(&self, key: &[u8]) -> Option<&[u8]> {
        self.table.get(key).unwrap()
    }

    fn range_from<'a>(&'a self, key: &'a [u8]) -> Self::Iterator<'a> {
        let iter = self.table.range(key..).unwrap();
        RedbBenchIterator { iter }
    }
}

pub struct RedbBenchIterator<'a> {
    iter: redb::RangeIter<'a, [u8], [u8]>,
}

impl BenchIterator for RedbBenchIterator<'_> {
    type Output<'a> = &'a [u8]
    where
        Self: 'a;

    fn next(&mut self) -> Option<(Self::Output<'_>, Self::Output<'_>)> {
        self.iter.next()
    }
}

pub struct RedbBenchWriteTransaction<'db> {
    txn: redb::WriteTransaction<'db>,
}

impl<'db> BenchWriteTransaction for RedbBenchWriteTransaction<'db> {
    type W<'txn> = RedbBenchInserter<'db, 'txn>
    where
        Self: 'txn;

    fn get_inserter(&mut self) -> Self::W<'_> {
        let table = self.txn.open_table(X).unwrap();
        RedbBenchInserter { table }
    }

    fn commit(self) -> Result<(), ()> {
        self.txn.commit().map_err(|_| ())
    }
}

pub struct RedbBenchInserter<'db, 'txn> {
    table: redb::Table<'db, 'txn, [u8], [u8]>,
}

impl BenchInserter for RedbBenchInserter<'_, '_> {
    fn insert(&mut self, key: &[u8], value: &[u8]) -> Result<(), ()> {
        self.table.insert(key, value).map_err(|_| ())
    }

    fn remove(&mut self, key: &[u8]) -> Result<(), ()> {
        self.table.remove(key).map(|_| ()).map_err(|_| ())
    }
}

pub struct SledBenchDatabase<'a> {
    db: &'a sled::Db,
    db_dir: &'a Path,
}

impl<'a> SledBenchDatabase<'a> {
    pub fn new(db: &'a sled::Db, path: &'a Path) -> Self {
        SledBenchDatabase { db, db_dir: path }
    }
}

impl<'a> BenchDatabase for SledBenchDatabase<'a> {
    type W<'db> = SledBenchWriteTransaction<'db>
    where
        Self: 'db;
    type R<'db> = SledBenchReadTransaction<'db>
    where
        Self: 'db;

    fn db_type_name() -> &'static str {
        "sled"
    }

    fn write_transaction(&mut self) -> Self::W<'_> {
        SledBenchWriteTransaction {
            db: self.db,
            db_dir: self.db_dir,
        }
    }

    fn read_transaction(&self) -> Self::R<'_> {
        SledBenchReadTransaction { db: self.db }
    }
}

pub struct SledBenchReadTransaction<'db> {
    db: &'db sled::Db,
}

impl<'db, 'b> BenchReadTransaction for SledBenchReadTransaction<'db> {
    type T<'txn> = SledBenchReader<'db>
    where
        Self: 'txn;

    fn get_reader(&self) -> Self::T<'_> {
        SledBenchReader { db: self.db }
    }
}

pub struct SledBenchReader<'db> {
    db: &'db sled::Db,
}

impl<'db> BenchReader for SledBenchReader<'db> {
    type Output<'r> = sled::IVec
    where
        Self: 'r;
    type Iterator<'r> = SledBenchIterator
    where
        Self: 'r;

    fn get(&self, key: &[u8]) -> Option<sled::IVec> {
        self.db.get(key).unwrap()
    }

    fn range_from<'a>(&'a self, key: &'a [u8]) -> Self::Iterator<'a> {
        let iter = self.db.range(key..);
        SledBenchIterator { iter }
    }
}

pub struct SledBenchIterator {
    iter: sled::Iter,
}

impl BenchIterator for SledBenchIterator {
    type Output<'a> = sled::IVec
    where
        Self: 'a;

    fn next(&mut self) -> Option<(Self::Output<'_>, Self::Output<'_>)> {
        self.iter.next().map(|x| x.unwrap())
    }
}

pub struct SledBenchWriteTransaction<'a> {
    db: &'a sled::Db,
    db_dir: &'a Path,
}

impl<'a> BenchWriteTransaction for SledBenchWriteTransaction<'a> {
    type W<'txn> = SledBenchInserter<'txn>
    where
        Self: 'txn;

    fn get_inserter(&mut self) -> Self::W<'_> {
        SledBenchInserter { db: self.db }
    }

    fn commit(self) -> Result<(), ()> {
        self.db.flush().unwrap();
        // Workaround for sled durability
        // Fsync all the files, because sled doesn't guarantee durability (it uses sync_file_range())
        // See: https://github.com/spacejam/sled/issues/1351
        for entry in fs::read_dir(self.db_dir).unwrap() {
            let entry = entry.unwrap();
            if entry.path().is_file() {
                let file = File::open(entry.path()).unwrap();
                file.sync_all().unwrap();
            }
        }
        Ok(())
    }
}

pub struct SledBenchInserter<'a> {
    db: &'a sled::Db,
}

impl<'a> BenchInserter for SledBenchInserter<'a> {
    fn insert(&mut self, key: &[u8], value: &[u8]) -> Result<(), ()> {
        self.db.insert(key, value).map(|_| ()).map_err(|_| ())
    }

    fn remove(&mut self, key: &[u8]) -> Result<(), ()> {
        self.db.remove(key).map(|_| ()).map_err(|_| ())
    }
}

pub struct LmdbRkvBenchDatabase<'a> {
    env: &'a lmdb::Environment,
    db: lmdb::Database,
}

impl<'a> LmdbRkvBenchDatabase<'a> {
    pub fn new(env: &'a lmdb::Environment) -> Self {
        let db = env.open_db(None).unwrap();
        LmdbRkvBenchDatabase { env, db }
    }
}

impl<'a> BenchDatabase for LmdbRkvBenchDatabase<'a> {
    type W<'db> = LmdbRkvBenchWriteTransaction<'db>
    where
        Self: 'db;
    type R<'db> = LmdbRkvBenchReadTransaction<'db>
    where
        Self: 'db;

    fn db_type_name() -> &'static str {
        "lmdb-rkv"
    }

    fn write_transaction(&mut self) -> Self::W<'_> {
        let txn = self.env.begin_rw_txn().unwrap();
        LmdbRkvBenchWriteTransaction { db: self.db, txn }
    }

    fn read_transaction(&self) -> Self::R<'_> {
        let txn = self.env.begin_ro_txn().unwrap();
        LmdbRkvBenchReadTransaction { db: self.db, txn }
    }
}

pub struct LmdbRkvBenchWriteTransaction<'db> {
    db: lmdb::Database,
    txn: lmdb::RwTransaction<'db>,
}

impl<'db> BenchWriteTransaction for LmdbRkvBenchWriteTransaction<'db> {
    type W<'txn> = LmdbRkvBenchInserter<'txn, 'db>
    where
        Self: 'txn;

    fn get_inserter(&mut self) -> Self::W<'_> {
        LmdbRkvBenchInserter {
            db: self.db,
            txn: &mut self.txn,
        }
    }

    fn commit(self) -> Result<(), ()> {
        use lmdb::Transaction;
        self.txn.commit().map_err(|_| ())
    }
}

pub struct LmdbRkvBenchInserter<'txn, 'db> {
    db: lmdb::Database,
    txn: &'txn mut lmdb::RwTransaction<'db>,
}

impl BenchInserter for LmdbRkvBenchInserter<'_, '_> {
    fn insert(&mut self, key: &[u8], value: &[u8]) -> Result<(), ()> {
        self.txn
            .put(self.db, &key, &value, lmdb::WriteFlags::empty())
            .map_err(|_| ())
    }

    fn remove(&mut self, key: &[u8]) -> Result<(), ()> {
        self.txn.del(self.db, &key, None).map_err(|_| ())
    }
}

pub struct LmdbRkvBenchReadTransaction<'db> {
    db: lmdb::Database,
    txn: lmdb::RoTransaction<'db>,
}

impl<'db> BenchReadTransaction for LmdbRkvBenchReadTransaction<'db> {
    type T<'txn> = LmdbRkvBenchReader<'txn, 'db>
    where
        Self: 'txn;

    fn get_reader(&self) -> Self::T<'_> {
        LmdbRkvBenchReader {
            db: self.db,
            txn: &self.txn,
        }
    }
}

pub struct LmdbRkvBenchReader<'txn, 'db> {
    db: lmdb::Database,
    txn: &'txn lmdb::RoTransaction<'db>,
}

impl<'txn, 'db> BenchReader for LmdbRkvBenchReader<'txn, 'db> {
    type Output<'b> = &'b [u8]
    where
        Self: 'b;
    type Iterator<'a> = LmdbRkvBenchIterator<'a>
    where
        Self: 'a;

    fn get(&self, key: &[u8]) -> Option<&[u8]> {
        use lmdb::Transaction;
        self.txn.get(self.db, &key).ok()
    }

    fn range_from<'a>(&'a self, key: &'a [u8]) -> Self::Iterator<'a> {
        use lmdb::{Cursor, Transaction};
        let iter = self.txn.open_ro_cursor(self.db).unwrap().iter_from(key);

        LmdbRkvBenchIterator { iter }
    }
}

pub struct LmdbRkvBenchIterator<'a> {
    iter: lmdb::Iter<'a>,
}

impl BenchIterator for LmdbRkvBenchIterator<'_> {
    type Output<'a> = &'a [u8]
    where
    Self: 'a;

    fn next(&mut self) -> Option<(Self::Output<'_>, Self::Output<'_>)> {
        self.iter.next().map(|x| x.unwrap())
    }
}
