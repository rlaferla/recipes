//! Service and ServiceFactory implementation. Specialized wrapper over substrate service.

use std::sync::Arc;
use sc_consensus::LongestChain;
use sc_service::{error::{Error as ServiceError}, AbstractService, Configuration, ServiceBuilder};
use sp_inherents::InherentDataProviders;
use sc_executor::native_executor_instance;
pub use sc_executor::NativeExecutor;
use sc_network::config::DummyFinalityProofRequestBuilder;

// Our native executor instance.
native_executor_instance!(
	pub Executor,
	runtime::api::dispatch,
	runtime::native_version,
);

/// Starts a `ServiceBuilder` for a full service.
///
/// Use this macro if you don't actually need the full service, but just the builder in order to
/// be able to perform chain operations.
macro_rules! new_full_start {
	($config:expr) => {{
		// A type alias we'll use for adding our RPC extension
		type RpcExtension = jsonrpc_core::IoHandler<sc_rpc::Metadata>;

		let builder = sc_service::ServiceBuilder::new_full::<
			runtime::opaque::Block, runtime::RuntimeApi, crate::service::Executor
		>($config)?
			.with_select_chain(|_config, backend| {
				Ok(sc_consensus::LongestChain::new(backend.clone()))
			})?
			.with_transaction_pool(|config, client, _fetcher, prometheus_registry| {
				let pool_api = sc_transaction_pool::FullChainApi::new(client.clone());
				Ok(sc_transaction_pool::BasicPool::new(config, std::sync::Arc::new(pool_api), prometheus_registry))
			})?
			.with_import_queue(|_config, client, _select_chain, _transaction_pool, spawn_task_handle| {
				Ok(sc_consensus_manual_seal::import_queue(
					Box::new(client),
					spawn_task_handle,
				))
			})?
			.with_rpc_extensions(|builder| -> Result<RpcExtension, _> {
				// Make an io handler to be extended with individual RPCs
				let mut io = jsonrpc_core::IoHandler::default();

				// Add the first rpc extension
				// Use the fully qualified name starting from `crate` because we're in macro_rules!
				io.extend_with(crate::silly_rpc::SillyRpc::to_delegate(crate::silly_rpc::Silly{}));

				// Add the second RPC extension
				// Because this one calls a Runtime API it needs a reference to the client.
				io.extend_with(sum_storage_rpc::SumStorageApi::to_delegate(sum_storage_rpc::SumStorage::new(builder.client().clone())));

				Ok(io)
			})?;

		builder
	}}
}

/// Builds a new service for a full client.
pub fn new_full(config: Configuration)
	-> Result<impl AbstractService, ServiceError>
{
	let is_authority = config.role.is_authority();

	//TODO This isn't great. It includes the timestamp inherent in all blocks
	// regardless of runtime.
	let inherent_data_providers = InherentDataProviders::new();
	inherent_data_providers
		.register_provider(sp_timestamp::InherentDataProvider)
		.map_err(Into::into)
		.map_err(sp_consensus::error::Error::InherentData)?;

	let builder = new_full_start!(config);
	let service = builder.build()?;

	if is_authority {
		let proposer = sc_basic_authorship::ProposerFactory::new(
			service.client(),
			service.transaction_pool(),
		);

		let authorship_future = sc_consensus_manual_seal::run_instant_seal(
			Box::new(service.client()),
			proposer,
			service.client().clone(),
			service.transaction_pool().pool().clone(),
			service.select_chain().ok_or(ServiceError::SelectChainRequired)?,
			inherent_data_providers
		);

		service.spawn_essential_task("instant-seal", authorship_future);
	};

	Ok(service)
}

/// Builds a new service for a light client.
pub fn new_light(config: Configuration)
	-> Result<impl AbstractService, ServiceError>
{
	ServiceBuilder::new_light::<runtime::opaque::Block, runtime::RuntimeApi, Executor>(config)?
		.with_select_chain(|_config, backend| {
			Ok(LongestChain::new(backend.clone()))
		})?
		.with_transaction_pool(|config, client, fetcher, prometheus_registry| {
			let fetcher = fetcher
				.ok_or_else(|| "Trying to start light transaction pool without active fetcher")?;
			let pool_api = sc_transaction_pool::LightChainApi::new(client.clone(), fetcher.clone());
			let pool = sc_transaction_pool::BasicPool::with_revalidation_type(
				config, Arc::new(pool_api), prometheus_registry, sc_transaction_pool::RevalidationType::Light,
			);
			Ok(pool)
		})?
		.with_import_queue_and_fprb(|_config, client, _backend, _fetcher, _select_chain, _tx_pool, spawn_task_handle| {
			let finality_proof_request_builder =
				Box::new(DummyFinalityProofRequestBuilder::default()) as Box<_>;

			let import_queue = sc_consensus_manual_seal::import_queue(
				Box::new(client),
				spawn_task_handle,
			);

			Ok((import_queue, finality_proof_request_builder))
		})?
		.with_finality_proof_provider(|_client, _backend| {
			Ok(Arc::new(()) as _)
		})?
		.build()
}