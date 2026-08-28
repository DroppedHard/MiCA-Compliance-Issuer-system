// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.28;

import {IERC20} from "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import {SafeERC20} from "@openzeppelin/contracts/token/ERC20/utils/SafeERC20.sol";

/// @title CaspDepositRouter
/// @notice Demo gateway for attributing an external rUSD deposit to one CASP ledger client.
/// @dev The sender approves this router first. Tokens move directly to CASP hot custody;
/// the client reference is emitted as evidence for the off-chain CASP observer.
contract CaspDepositRouter {
    using SafeERC20 for IERC20;

    IERC20 public immutable token;
    address public immutable custodyWallet;

    error ZeroAddress();
    error ZeroAmount();
    error EmptyClientReference();

    event DepositReceived(
        address indexed sender,
        bytes32 indexed clientReference,
        uint256 amount
    );

    constructor(IERC20 token_, address custodyWallet_) {
        if (address(token_) == address(0) || custodyWallet_ == address(0)) revert ZeroAddress();
        token = token_;
        custodyWallet = custodyWallet_;
    }

    function depositFor(bytes32 clientReference, uint256 amount) external {
        if (clientReference == bytes32(0)) revert EmptyClientReference();
        if (amount == 0) revert ZeroAmount();
        token.safeTransferFrom(msg.sender, custodyWallet, amount);
        emit DepositReceived(msg.sender, clientReference, amount);
    }
}
