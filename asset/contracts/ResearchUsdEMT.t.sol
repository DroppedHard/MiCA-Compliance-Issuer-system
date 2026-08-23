// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.28;

import {Test} from "forge-std/Test.sol";
import {IAccessControl} from "@openzeppelin/contracts/access/IAccessControl.sol";
import {Pausable} from "@openzeppelin/contracts/utils/Pausable.sol";
import {ResearchUsdEMT} from "./ResearchUsdEMT.sol";

contract ResearchUsdEMTTest is Test {
    ResearchUsdEMT token;
    address admin = makeAddr("admin");
    address alice = makeAddr("alice");
    address bob = makeAddr("bob");
    address outsider = makeAddr("outsider");

    function setUp() public { token = new ResearchUsdEMT(admin); }

    function test_MetadataAndInitialState() public view {
        assertEq(token.name(), "Research USD EMT");
        assertEq(token.symbol(), "rUSD");
        assertEq(token.decimals(), 6);
        assertEq(token.totalSupply(), 0);
        assertTrue(token.hasRole(token.DEFAULT_ADMIN_ROLE(), admin));
    }

    function test_AdminCanMintAndBurnForRedemption() public {
        vm.startPrank(admin);
        token.mint(alice, 125_000_000);
        token.burn(alice, 25_000_000);
        vm.stopPrank();
        assertEq(token.balanceOf(alice), 100_000_000);
        assertEq(token.totalSupply(), 100_000_000);
    }

    function test_UnauthorizedAccountCannotMint() public {
        vm.expectRevert(abi.encodeWithSelector(IAccessControl.AccessControlUnauthorizedAccount.selector, outsider, token.MINTER_ROLE()));
        vm.prank(outsider);
        token.mint(alice, 1);
    }

    function test_PauseStopsTransfersMintingAndBurning() public {
        vm.startPrank(admin);
        token.mint(alice, 10_000_000);
        token.pause();
        vm.expectRevert(Pausable.EnforcedPause.selector);
        token.mint(bob, 1);
        vm.expectRevert(Pausable.EnforcedPause.selector);
        token.burn(alice, 1);
        vm.stopPrank();
        vm.prank(alice);
        vm.expectRevert(Pausable.EnforcedPause.selector);
        token.transfer(bob, 1);
    }

    function test_FrozenAccountCannotSendReceiveMintOrBurn() public {
        vm.startPrank(admin);
        token.mint(alice, 10_000_000);
        token.freeze(alice);
        vm.expectRevert(abi.encodeWithSelector(ResearchUsdEMT.AccountFrozen.selector, alice));
        token.mint(alice, 1);
        vm.expectRevert(abi.encodeWithSelector(ResearchUsdEMT.AccountFrozen.selector, alice));
        token.burn(alice, 1);
        vm.stopPrank();
        vm.prank(alice);
        vm.expectRevert(abi.encodeWithSelector(ResearchUsdEMT.AccountFrozen.selector, alice));
        token.transfer(bob, 1);
        vm.prank(bob);
        vm.expectRevert(abi.encodeWithSelector(ResearchUsdEMT.AccountFrozen.selector, alice));
        token.transfer(alice, 1);
    }

    function test_UnfreezeRestoresTransfers() public {
        vm.startPrank(admin);
        token.mint(alice, 2_000_000);
        token.freeze(alice);
        token.unfreeze(alice);
        vm.stopPrank();
        vm.prank(alice);
        token.transfer(bob, 1_000_000);
        assertEq(token.balanceOf(bob), 1_000_000);
    }
}
