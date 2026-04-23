use crate::authority::PublisherAuthority;

#[derive(Debug)]
pub struct AuthorityServer {
    authority: PublisherAuthority,
}

impl AuthorityServer {
    pub fn new(authority: PublisherAuthority) -> Self {
        Self { authority }
    }

    pub fn authority(&self) -> &PublisherAuthority {
        &self.authority
    }

    pub fn authority_mut(&mut self) -> &mut PublisherAuthority {
        &mut self.authority
    }

    pub fn into_inner(self) -> PublisherAuthority {
        self.authority
    }
}
