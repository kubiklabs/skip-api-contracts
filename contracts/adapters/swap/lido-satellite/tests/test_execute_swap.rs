use cosmwasm_std::{
    testing::{mock_dependencies, mock_env, mock_info},
    to_binary, Addr, Coin,
    ReplyOn::Never,
    SubMsg, WasmMsg,
};
use lido_satellite::msg::ExecuteMsg as LidoSatelliteExecuteMsg;
use skip::swap::{ExecuteMsg, SwapOperation};
use skip_api_swap_adapter_lido_satellite::{
    error::{ContractError, ContractResult},
    state::{
        BRIDGED_DENOM, CANONICAL_DENOM, ENTRY_POINT_CONTRACT_ADDRESS,
        LIDO_SATELLITE_CONTRACT_ADDRESS,
    },
};
use test_case::test_case;

/*
Test Cases:

Expect Success
    - 'Mint' Swap Operation
    - 'Burn' Swap Operation

Expect Error
    - Incorrect denom
    - Unauthorized Caller (Only the stored entry point contract can call this function)
    - No Coin Sent
    - More Than One Coin Sent

 */

// Define test parameters
struct Params {
    caller: String,
    info_funds: Vec<Coin>,
    swap_operations: Vec<SwapOperation>,
    expected_messages: Vec<SubMsg>,
    expected_error: Option<ContractError>,
}

// Test execute_swap
#[test_case(
    Params {
        caller: "entry_point".to_string(),
        info_funds: vec![Coin::new(100, "ibc/wstETH")],
        swap_operations: vec![],
        expected_messages: vec![
            SubMsg {
                id: 0,
                msg: WasmMsg::Execute {
                    contract_addr: "lido_satellite_contract".to_string(),
                    msg: to_binary(&LidoSatelliteExecuteMsg::Mint{ receiver: None })?,
                    funds: vec![Coin::new(100, "ibc/wstETH")],
                }
                .into(),
                gas_limit: None,
                reply_on: Never,
            },
            SubMsg {
                id: 0,
                msg: WasmMsg::Execute {
                    contract_addr: "swap_contract_address".to_string(),
                    msg: to_binary(&ExecuteMsg::TransferFundsBack {
                        swapper: Addr::unchecked("entry_point"),
                        return_denom: String::from("factory/wstETH"),
                    })?,
                    funds: vec![],
                }
                .into(),
                gas_limit: None,
                reply_on: Never,
            },
        ],
        expected_error: None,
    };
    "'Mint' Swap Operation")]
#[test_case(
    Params {
        caller: "entry_point".to_string(),
        info_funds: vec![Coin::new(100, "factory/wstETH")],
        swap_operations: vec![],
        expected_messages: vec![
            SubMsg {
                id: 0,
                msg: WasmMsg::Execute {
                    contract_addr: "lido_satellite_contract".to_string(),
                    msg: to_binary(&LidoSatelliteExecuteMsg::Burn{ receiver: None })?,
                    funds: vec![Coin::new(100, "factory/wstETH")],
                }
                .into(),
                gas_limit: None,
                reply_on: Never,
            },
            SubMsg {
                id: 0,
                msg: WasmMsg::Execute {
                    contract_addr: "swap_contract_address".to_string(),
                    msg: to_binary(&ExecuteMsg::TransferFundsBack {
                        swapper: Addr::unchecked("entry_point"),
                        return_denom: String::from("ibc/wstETH"),
                    })?,
                    funds: vec![],
                }
                .into(),
                gas_limit: None,
                reply_on: Never,
            },
        ],
        expected_error: None,
    };
    "'Burn' Swap Operation")]
#[test_case(
    Params {
        caller: "entry_point".to_string(),
        info_funds: vec![Coin::new(100, "uosmo")],
        swap_operations: vec![],
        expected_messages: vec![],
        expected_error: Some(ContractError::UnsupportedDenom),
    };
    "Incorrect denom")]
#[test_case(
    Params {
        caller: "entry_point".to_string(),
        info_funds: vec![],
        swap_operations: vec![],
        expected_messages: vec![],
        expected_error: Some(ContractError::Payment(cw_utils::PaymentError::NoFunds{})),
    };
    "No Coin Sent - Expect Error")]
#[test_case(
    Params {
        caller: "entry_point".to_string(),
        info_funds: vec![
            Coin::new(100, "untrn"),
            Coin::new(100, "uosmo"),
        ],
        swap_operations: vec![],
        expected_messages: vec![],
        expected_error: Some(ContractError::Payment(cw_utils::PaymentError::MultipleDenoms{})),
    };
    "More Than One Coin Sent - Expect Error")]
#[test_case(
    Params {
        caller: "random".to_string(),
        info_funds: vec![
            Coin::new(100, "untrn"),
            Coin::new(100, "uosmo"),
        ],
        swap_operations: vec![],
        expected_messages: vec![],
        expected_error: Some(ContractError::Unauthorized),
    };
    "Unauthorized Caller - Expect Error")]
fn test_execute_swap(params: Params) -> ContractResult<()> {
    // Create mock dependencies
    let mut deps = mock_dependencies();

    // Create mock env
    let mut env = mock_env();
    env.contract.address = Addr::unchecked("swap_contract_address");

    // Convert info funds vector into a slice of Coin objects
    let info_funds: &[Coin] = &params.info_funds;

    // Create mock info with entry point contract address
    let info = mock_info(&params.caller, info_funds);

    // Store the entry point contract address
    ENTRY_POINT_CONTRACT_ADDRESS.save(deps.as_mut().storage, &Addr::unchecked("entry_point"))?;

    // Store the router contract address
    LIDO_SATELLITE_CONTRACT_ADDRESS.save(
        deps.as_mut().storage,
        &Addr::unchecked("lido_satellite_contract"),
    )?;

    // Store Lido Satellite denoms
    BRIDGED_DENOM.save(deps.as_mut().storage, &String::from("ibc/wstETH"))?;
    CANONICAL_DENOM.save(deps.as_mut().storage, &String::from("factory/wstETH"))?;

    // Call execute_swap with the given test parameters
    let res = skip_api_swap_adapter_lido_satellite::contract::execute(
        deps.as_mut(),
        env,
        info,
        ExecuteMsg::Swap {
            operations: params.swap_operations.clone(),
        },
    );

    // Assert the behavior is correct
    match res {
        Ok(res) => {
            // Assert the test did not expect an error
            assert!(
                params.expected_error.is_none(),
                "expected test to error with {:?}, but it succeeded",
                params.expected_error
            );

            // Assert the messages are correct
            assert_eq!(res.messages, params.expected_messages);
        }
        Err(err) => {
            // Assert the test expected an error
            assert!(
                params.expected_error.is_some(),
                "expected test to succeed, but it errored with {:?}",
                err
            );

            // Assert the error is correct
            assert_eq!(err, params.expected_error.unwrap());
        }
    }

    Ok(())
}
