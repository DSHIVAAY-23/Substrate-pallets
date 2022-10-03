#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
	// use frame_support::{
	// 	dispatch::{DipatchResult, DispatchResultWithPostInfo},
	// 	pallet_prelude::*,
	// 	sp_runtime::traits::{Hash, Zero},
	// 	traits::{Currency, ExistenceRequirement, Randomness},
	// };

	// use frame_support::prelude::*;
	use frame_support::{
		pallet_prelude::*,
		sp_runtime::traits::Hash,
		traits::{tokens::ExistenceRequirement, Currency, Randomness},
		transactional,
	};
	use frame_system::pallet_prelude::*;
	use scale_info::TypeInfo;
	use sp_io::hashing::blake2_128;

	#[cfg(feature = "std")]
	use frame_support::serde::{Deserialize, Serialize};

	type AccountOf<T> = <T as frame_system::Config>::AccountId;
	type BalanceOf<T> =
		<<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

	/// Struct for holding Nft information
	#[derive(Clone, Encode, Decode, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
	#[scale_info(skip_type_params(T))]
	#[codec(mel_bound())]
	pub struct Nft<T: Config> {
		pub dna: [u8; 16],
		pub price: Option<BalanceOf<T>>,
		pub owner: AccountOf<T>,
	}

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
		/// The currency handler for the Nfts pallet
		type Currency: Currency<Self::AccountId>;
		type NftRandomness: Randomness<Self::Hash, Self::BlockNumber>;

		#[pallet::constant]
		type MaxNftOwned: Get<u32>;
	}

	// Errors
	#[pallet::error]
	pub enum Error<T> {
		NftCountOverflow,
		/// An account cannot own more APes than `MaxNftCount`.
		ExceedMaxNftOwned,
		/// Buyer cannot be the owner.
		BuyerIsNftOwner,
		/// Cannot transfer a nft to its owner.
		TransferToSelf,
		/// This nft already exists
		NftExists,
		/// This nft doesn't exist
		NftNotExist,
		/// Handles checking that the nft is owned by the account transferring, buying or setting a price for it.
		NotNftOwner,
		/// Ensures the Nft is for sale.
		NftNotForSale,
		/// Ensures that the buying price is greater than the asking price.
		NftBidPriceTooLow,
		/// Ensures that an account has enough funds to purchase a Nft.
		NotEnoughBalance,
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		Created(T::AccountId, T::Hash),
		PriceSet(T::AccountId, T::Hash, Option<BalanceOf<T>>),
		Transferred(T::AccountId, T::AccountId, T::Hash),
		Bought(T::AccountId, T::AccountId, T::Hash, BalanceOf<T>),
	}

	#[pallet::storage]
	#[pallet::getter(fn nft_count)]
	/// Keeps track of the number of Nfts in existence.
	pub(super) type NftCount<T: Config> = StorageValue<_, u64, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn nfts)]
	pub(super) type Nfts<T: Config> = StorageMap<_, Twox64Concat, T::Hash, Nft<T>>;

	#[pallet::storage]
	#[pallet::getter(fn nfts_owned)]
	/// Keeps track of what accounts own what Nft.
	pub(super) type NftsOwned<T: Config> =
		StorageMap<_, Twox64Concat, T::AccountId, BoundedVec<T::Hash, T::MaxNftOwned>, ValueQuery>;

	// Our pallet's genesis configuration.
	#[pallet::genesis_config]
	pub struct GenesisConfig<T: Config> {
		pub nfts: Vec<(T::AccountId, [u8; 16])>,
	}

	// Required to implement default for GenesisConfig.
	#[cfg(feature = "std")]
	impl<T: Config> Default for GenesisConfig<T> {
		fn default() -> GenesisConfig<T> {
			GenesisConfig { nfts: vec![] }
		}
	}

	#[pallet::genesis_build]
	impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
		fn build(&self) {
			// When building a kitty from genesis config, we require the dna and gender to be supplied.
			for (account, dna) in &self.nfts {
				let _ = <Pallet<T>>::mint(account, Some(dna.clone()));
			}
		}
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(100)]
		pub fn create_nft(origin: OriginFor<T>) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			let nft_id = Self::mint(&sender, None)?;

			log::info!("An nft is born with ID: {:?}.", nft_id);

			Self::deposit_event(Event::Created(sender, nft_id));

			Ok(())
		}
		/// Updates Nft price and updates storage.
		#[pallet::weight(100)]
		pub fn set_price(
			origin: OriginFor<T>,
			nft_id: T::Hash,
			new_price: Option<BalanceOf<T>>,
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;

			// Ensure the kitty exists and is called by the kitty owner
			ensure!(Self::is_nft_owner(&nft_id, &sender)?, <Error<T>>::NotNftOwner);

			let mut nft = Self::nfts(&nft_id).ok_or(<Error<T>>::NftNotExist)?;

			nft.price = new_price.clone();
			<Nfts<T>>::insert(&nft_id, nft);

			// Deposit a "PriceSet" event.
			Self::deposit_event(Event::PriceSet(sender, nft_id, new_price));

			Ok(())
		}
		/// Any account that holds an nft can send it to another Account. This will reset the asking
		/// price of the nft, marking it not for sale.
		#[pallet::weight(100)]
		pub fn transfer(origin: OriginFor<T>, to: T::AccountId, nft_id: T::Hash) -> DispatchResult {
			let from = ensure_signed(origin)?;

			// Ensure the kitty exists and is called by the kitty owner
			ensure!(Self::is_nft_owner(&nft_id, &from)?, <Error<T>>::NotNftOwner);

			// Verify the kitty is not transferring back to its owner.
			ensure!(from != to, <Error<T>>::TransferToSelf);

			// Verify the recipient has the capacity to receive one more kitty
			let to_owned = <NftsOwned<T>>::get(&to);
			ensure!((to_owned.len() as u32) < T::MaxNftOwned::get(), <Error<T>>::ExceedMaxNftOwned);

			Self::transfer_nft_to(&nft_id, &to)?;

			Self::deposit_event(Event::Transferred(from, to, nft_id));

			Ok(())
		}
		/// Buy a saleable Nft. The bid price provided from the buyer has to be equal or higher
		/// than the ask price from the seller.
		/// Marking this method `transactional` so when an error is returned, we ensure no storage is changed.
		#[transactional]
		#[pallet::weight(100)]
		pub fn buy_nft(
			origin: OriginFor<T>,
			nft_id: T::Hash,
			bid_price: BalanceOf<T>,
		) -> DispatchResult {
			let buyer = ensure_signed(origin)?;

			// Check the nft exists and buyer is not the current nft owner
			let nft = Self::nfts(&nft_id).ok_or(<Error<T>>::NftNotExist)?;
			ensure!(nft.owner != buyer, <Error<T>>::BuyerIsNftOwner);

			// Check the nft is for sale and the nft ask price <= bid_price
			if let Some(ask_price) = nft.price {
				ensure!(ask_price <= bid_price, <Error<T>>::NftBidPriceTooLow);
			} else {
				Err(<Error<T>>::NftNotForSale)?;
			}

			// Check the buyer has enough free balance
			ensure!(T::Currency::free_balance(&buyer) >= bid_price, <Error<T>>::NotEnoughBalance);

			// Verify the buyer has the capacity to receive one more kitty
			let to_owned = <NftsOwned<T>>::get(&buyer);
			ensure!((to_owned.len() as u32) < T::MaxNftOwned::get(), <Error<T>>::ExceedMaxNftOwned);

			let seller = nft.owner.clone();

			// Transfer the amount from buyer to seller
			T::Currency::transfer(&buyer, &seller, bid_price, ExistenceRequirement::KeepAlive)?;

			// Transfer the kitty from seller to buyer
			Self::transfer_nft_to(&nft_id, &buyer)?;

			Self::deposit_event(Event::Bought(buyer, seller, nft_id, bid_price));

			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
		fn gen_dna() -> [u8; 16] {
			let payload = (
				T::NftRandomness::random(&b"dna"[..]).0,
				<frame_system::Pallet<T>>::extrinsic_index().unwrap_or_default(),
				<frame_system::Pallet<T>>::block_number(),
			);
			payload.using_encoded(blake2_128)
		}

		// Helper to mint an nft.
		pub fn mint(owner: &T::AccountId, dna: Option<[u8; 16]>) -> Result<T::Hash, Error<T>> {
			let nft = Nft::<T> {
				dna: dna.unwrap_or_else(Self::gen_dna),
				price: None,
				owner: owner.clone(),
			};

			let nft_id = T::Hashing::hash_of(&nft);

			// Performs this operation first as it may fail
			let new_count = Self::nft_count().checked_add(1).ok_or(<Error<T>>::NftCountOverflow)?;

			// Check if the kitty does not already exist in our storage map
			ensure!(Self::nfts(&nft_id) == None, <Error<T>>::NftExists);

			// Performs this operation first because as it may fail
			<NftsOwned<T>>::try_mutate(&owner, |nft_vec| nft_vec.try_push(nft_id))
				.map_err(|_| <Error<T>>::ExceedMaxNftOwned)?;

			<Nfts<T>>::insert(nft_id, nft);
			<NftCount<T>>::put(new_count);
			Ok(nft_id)
		}

		pub fn is_nft_owner(nft_id: &T::Hash, account: &T::AccountId) -> Result<bool, Error<T>> {
			match Self::nfts(nft_id) {
				Some(nft) => Ok(nft.owner == *account),
				None => Err(<Error<T>>::NftNotExist),
			}
		}
		#[transactional]
		pub fn transfer_nft_to(nft_id: &T::Hash, to: &T::AccountId) -> Result<(), Error<T>> {
			let mut nft = Self::nfts(&nft_id).ok_or(<Error<T>>::NftNotExist)?;

			let prev_owner = nft.owner.clone();

			// Remove `kitty_id` from the KittyOwned vector of `prev_kitty_owner`
			<NftsOwned<T>>::try_mutate(&prev_owner, |owned| {
				if let Some(ind) = owned.iter().position(|&id| id == *nft_id) {
					owned.swap_remove(ind);
					return Ok(());
				}
				Err(())
			})
			.map_err(|_| <Error<T>>::NftNotExist)?;

			// Update the kitty owner
			nft.owner = to.clone();
			// Reset the ask price so the kitty is not for sale until `set_price()` is called
			// by the current owner.
			nft.price = None;

			<Nfts<T>>::insert(nft_id, nft);

			<NftsOwned<T>>::try_mutate(to, |vec| vec.try_push(*nft_id))
				.map_err(|_| <Error<T>>::ExceedMaxNftOwned)?;

			Ok(())
		}
	}
}