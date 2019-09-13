//! Ipld dag.
use crate::error::{format_err, Result};
use crate::hash::Hash;
use crate::ipld::{Cid, Ipld};
use crate::path::Path;
use crate::store::IpldStore;

/// Path in a dag.
#[derive(Clone, Debug, PartialEq, Hash)]
pub struct DagPath<'a>(&'a Cid, Path);

impl<'a> DagPath<'a> {
    /// Create a new dag path.
    pub fn new<T: Into<Path>>(cid: &'a Cid, path: T) -> Self {
        Self(cid, path.into())
    }
}

impl<'a> From<&'a Cid> for DagPath<'a> {
    fn from(cid: &'a Cid) -> Self {
        Self(cid, Default::default())
    }
}

/// The DAG.
#[derive(Debug)]
pub struct Dag<TStore: IpldStore> {
    store: TStore,
}

impl<TStore: IpldStore> Dag<TStore> {
    /// Creates a new Dag.
    pub fn new(store: TStore) -> Self {
        Self { store }
    }

    /// Retrives a block from the store.
    pub fn get_ipld(&self, cid: &Cid) -> Result<Ipld> {
        self.store.read_ipld(cid)
    }

    /// Retrives ipld from the dag.
    pub fn get(&self, path: &DagPath) -> Result<Option<Ipld>> {
        let mut root = self.store.read_ipld(&path.0)?;
        let mut ipld = &root;
        for segment in path.1.iter() {
            if let Some(next) = match ipld {
                Ipld::List(_) => {
                    let index: usize = segment.parse()?;
                    ipld.get(index)
                }
                Ipld::Map(_) => ipld.get(segment.as_str()),
                _ => return Err(format_err!("Cannot index into {:?}", ipld)),
            } {
                if let Ipld::Link(cid) = next {
                    root = self.store.read_ipld(cid)?;
                    ipld = &root;
                } else {
                    ipld = next;
                }
            } else {
                return Ok(None);
            }
        }
        Ok(Some(ipld.to_owned()))
    }

    /// Puts ipld into the dag.
    pub fn put_ipld<H: Hash>(&mut self, ipld: &Ipld) -> Result<Cid> {
        self.store.write_cbor::<H, _>(ipld)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hash::Blake2b;
    use crate::ipld;
    use crate::store::mock::MemStore;

    #[test]
    fn test_dag() {
        let store = MemStore::default();
        let mut dag = Dag::new(store);
        let cid = dag.put_ipld::<Blake2b>(&ipld!({"a": 3})).unwrap();
        let root = dag
            .put_ipld::<Blake2b>(&ipld!({"root": [{"child": &cid}]}))
            .unwrap();
        let path = DagPath::new(&root, "root/0/child/a");
        assert_eq!(dag.get(&path).unwrap(), Some(Ipld::Integer(3)));
    }
}
