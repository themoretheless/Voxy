//! Immutable validated pair order and CSR shared by liquid transport codecs.
use crate::LiquidDensityError;
#[derive(Debug)]
pub struct LiquidPairTopology {particles:usize,pairs:Vec<(usize,usize)>,csr:Vec<u32>}
impl LiquidPairTopology {
    pub fn new(particles:usize,pairs:&[(usize,usize)])->Result<Self,LiquidDensityError> {
        if particles==0 {return Err(LiquidDensityError::Shape);}
        let size=pairs.len().checked_mul(2).and_then(|n|n.checked_add(particles)).and_then(|n|n.checked_add(1))
            .filter(|&n|n<=16*1024*1024).ok_or(LiquidDensityError::Capacity)?;
        let mut seen=std::collections::HashSet::new();seen.try_reserve(pairs.len()).map_err(|_|LiquidDensityError::Capacity)?;
        let mut degrees=Vec::new();degrees.try_reserve_exact(particles).map_err(|_|LiquidDensityError::Capacity)?;degrees.resize(particles,0usize);
        for &(i,j) in pairs {
            if i>=j || j>=particles || !seen.insert((i,j)) {return Err(LiquidDensityError::Shape);}
            degrees[i]+=1;degrees[j]+=1;
        }
        let mut csr=Vec::new();csr.try_reserve_exact(size).map_err(|_|LiquidDensityError::Capacity)?;csr.push(0);
        let mut cursor=0;for degree in degrees {cursor+=degree;csr.push(cursor as u32);}
        let mut next=csr[..particles].to_vec();csr.resize(size,0);
        for (pair,&(i,j)) in pairs.iter().enumerate() {
            for particle in [i,j] {csr[particles+1+next[particle] as usize]=pair as u32;next[particle]+=1;}
        }
        let mut owned=Vec::new();owned.try_reserve_exact(pairs.len()).map_err(|_|LiquidDensityError::Capacity)?;owned.extend_from_slice(pairs);
        Ok(Self {particles,pairs:owned,csr})
    }
    pub fn particle_count(&self)->usize {self.particles}
    pub fn pairs(&self)->&[(usize,usize)] {&self.pairs}
    pub fn matches(&self,particles:usize,pairs:&[(usize,usize)])->bool {self.particles==particles && self.pairs==pairs}
    pub(crate) fn append_csr(&self,words:&mut Vec<u32>) {words.extend_from_slice(&self.csr);}
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn topology_preserves_original_per_particle_pair_order_and_rejects_invalid_pairs() {
        let pairs=[(1,3),(0,2),(0,1),(2,3)];let topology=LiquidPairTopology::new(5,&pairs).unwrap();
        assert_eq!(topology.csr,[0,2,4,6,8,8,1,2,0,2,1,3,0,3]);
        assert!(topology.matches(5,&pairs));assert!(!topology.matches(4,&pairs));
        let mut reverse=pairs;reverse.reverse();assert!(!topology.matches(5,&reverse));
        for pairs in [vec![(0,0)],vec![(1,0)],vec![(0,5)],vec![(0,1),(0,1)]] {assert!(LiquidPairTopology::new(5,&pairs).is_err());}
        assert!(LiquidPairTopology::new(0,&[]).is_err());assert!(LiquidPairTopology::new(usize::MAX,&[]).is_err());
        assert_eq!(LiquidPairTopology::new(3,&[]).unwrap().csr,[0,0,0,0]);
    }
}
