use crate::{
    query::{ComponentCollection, FilterCollection, Query, Res},
    resource::Resource,
};

pub trait SystemParams {
    fn print();
}

impl<T: ComponentCollection, F: FilterCollection> SystemParams for Query<T, F> {
    fn print() {
        println!(
            "{}\n{}",
            std::any::type_name::<T>(),
            std::any::type_name::<F>()
        );
    }
}

impl<R: Resource> SystemParams for Res<R> {
    fn print() {
        println!("{}", std::any::type_name::<R>());
    }
}

pub trait IntoSystemDescriptor<Params> {
    fn into_descriptor(&self) -> SystemDescriptor;
}

impl<F, Params: SystemParams> IntoSystemDescriptor<Params> for F
where
    F: FnMut(Params),
{
    fn into_descriptor(&self) -> SystemDescriptor {
        Params::print();

        SystemDescriptor {}
    }
}

impl<F, Params1: SystemParams, Params2: SystemParams> IntoSystemDescriptor<(Params1, Params2)> for F
where
    F: FnMut(Params1, Params2),
{
    fn into_descriptor(&self) -> SystemDescriptor {
        Params1::print();
        Params2::print();

        SystemDescriptor {}
    }
}

pub struct SystemDescriptor {}
